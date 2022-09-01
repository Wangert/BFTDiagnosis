use std::{collections::HashMap, error::Error};

use libp2p::{
    futures::StreamExt,
    gossipsub::{GossipsubEvent, IdentTopic},
    mdns::MdnsEvent,
    swarm::SwarmEvent,
    PeerId,
};
use network::{
    p2p_protocols::{
        base_behaviour::{BaseBehaviour, OutEvent},
        unicast::behaviour::UnicastEvent,
    },
    peer::Peer,
};

use tokio::io::{self, AsyncBufReadExt};
use utils::coder::{self, serialize_into_bytes};

use crate::{
    common::{generate_bls_keys, generate_consensus_requests_command},
    message::{Command, ConsensusNodePKInfo, CommandMessage, ConsensusStartData, ConsensusEndData, ConsensusDataMessage, ConsensusData, Component, InteractiveMessage, Message, TestItem},
};

use super::data_warehouse::DataWarehouse;


pub struct AnalysisNode {
    pub id: PeerId,
    pub peer: Peer,
    pub connected_nodes: HashMap<String, PeerId>,

    controller_id: PeerId,
    current_test_item: Option<TestItem>,

    data_warehouse: DataWarehouse,
}

impl AnalysisNode {
    pub fn new(peer: Peer, controller_id: PeerId, port: &str) -> Self {
        // let db_path = format!("./storage/data/{}_public_keys", port);
        Self {
            id: peer.id,
            peer,
            connected_nodes: HashMap::new(),
            controller_id,
            current_test_item: None,
            data_warehouse: DataWarehouse::new(),
        }
    }

    pub async fn peer_start(&mut self) -> Result<(), Box<dyn Error>> {
        self.peer_mut().swarm_start(false).await?;
        self.message_handler_start().await;

        Ok(())
    }

    pub fn id(&self) -> PeerId {
        self.id.clone()
    }

    pub fn id_bytes(&self) -> Vec<u8> {
        self.id.to_bytes()
    }

    pub fn peer_mut(&mut self) -> &mut Peer {
        &mut self.peer
    }

    pub fn add_controller(&mut self, controller_id: PeerId) {
        self.controller_id = controller_id;
    }

    pub fn set_test_item(&mut self, test_item: TestItem) {
        self.current_test_item = Some(test_item);
    }

    pub fn init(&mut self) {
        let id_bytes = self.id_bytes();
        let component = Component::Analyzer(id_bytes);
        let interactive_message = InteractiveMessage::ComponentInfo(component);
        let message = Message {
            interactive_message,
        };

        let serialized_message = coder::serialize_into_bytes(&message);
        let topic = IdentTopic::new("Initialization");
        if let Err(e) = self
            .peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), serialized_message)
        {
            eprintln!("Publish message error:{:?}", e);
        }
    }

    pub fn controller_message_handler(&mut self, message: Message) {
        match message.interactive_message {
            InteractiveMessage::ComponentInfo(Component::Controller(id_bytes)) => {
                let controller_id = PeerId::from_bytes(&id_bytes[..]).unwrap();
                self.add_controller(controller_id);
            },
            InteractiveMessage::ComponentInfo(Component::Analyzer(id_bytes)) => {

            },
            InteractiveMessage::TestItem(item) => {
                self.set_test_item(item);
            },
            _ => {}
        };
    }

    pub fn consensus_node_message_handler(&mut self, origin_peer_id: &PeerId, message: ConsensusDataMessage) {
        match message.data {
            ConsensusData::ConsensusStartData(data) => {
                println!("【ConsensusStartData(from {:?})】: {:?}", origin_peer_id.to_string(), &data);
                self.data_warehouse.store_consensus_start_data(*origin_peer_id, data);
            }
            ConsensusData::ConsensusEndData(data) => {
                println!("【ConsensusEndData(from {:?})】: {:?}", origin_peer_id.to_string(), &data);
                self.data_warehouse.store_consensus_end_data(*origin_peer_id, data);
            }
        };
    }

    pub async fn message_handler_start(&mut self) {
        let mut stdin = io::BufReader::new(io::stdin()).lines();

        loop {
            tokio::select! {
                line = stdin.next_line() => {
                    if let Ok(Some(command)) = line {
                        println!("{}", command);
                        let count = command.parse::<usize>().ok();
                        match count {
                            None => match &command as &str {
                                "Init" => {
                                    self.init();
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                }
                event = self.peer_mut().network_swarm_mut().select_next_some() => match event {
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        let peer_id = PeerId::from_bytes(&message.source[..]).unwrap();
                        if self.controller_id.to_string().eq(&peer_id.to_string()) {
                            let message: Message = coder::deserialize_for_bytes(&message.data);
                            self.controller_message_handler(message);
                        } else if self.connected_nodes.contains_key(&peer_id.to_string()) {
                            let consensus_data_message: ConsensusDataMessage = coder::deserialize_for_bytes(&message.data);
                            self.consensus_node_message_handler(&peer_id, consensus_data_message);
                        }
                    }
                    SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                        propagation_source: _peer_id,
                        message_id: _id,
                        message,
                    })) => {
                        let message: Message = coder::deserialize_for_bytes(&message.data);
                        match message.interactive_message {
                            InteractiveMessage::ComponentInfo(Component::Controller(id_bytes)) => {
                                let controller_id = PeerId::from_bytes(&id_bytes[..]).unwrap();
                                self.add_controller(controller_id);
                            },
                            InteractiveMessage::ComponentInfo(Component::Analyzer(id_bytes)) => {

                            },
                            _ => {}
                        };

                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                        let swarm = self.peer_mut().network_swarm_mut();
                        for (peer, _) in list {
                            println!("Discovered {:?}", &peer);
                            swarm.behaviour_mut().unicast.add_node_to_partial_view(&peer);
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                            // self.connected_nodes.insert(peer.to_string(), peer.clone());
                        }
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Expired(list))) => {
                        let swarm = self.peer_mut().network_swarm_mut();
                        for (peer, _) in list {
                            if !swarm.behaviour_mut().mdns.has_node(&peer) {
                                swarm.behaviour_mut().unicast.remove_node_from_partial_view(&peer);
                                swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer);
                                // self.connected_nodes.remove(&peer.to_string());
                            }
                        }
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on {:?}", address);
                    }
                    _ => {}
                }
            }
        }
    }
}
