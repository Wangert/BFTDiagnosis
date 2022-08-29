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
    message::{Command, ConsensusNodePKInfo, CommandMessage, Component, InteractiveMessage, Message},
};

use super::executor::Executor;

pub struct ControllerNode {
    id: PeerId,
    pub peer: Peer,
    pub executor: Executor,
    pub connected_nodes: HashMap<String, PeerId>,

    analysis_node_id: PeerId,
}

impl ControllerNode {
    pub fn new(peer: Peer, analysis_node_id: PeerId, port: &str) -> Self {
        let db_path = format!("./storage/data/{}_public_keys", port);
        let executor = Executor::new(&db_path);
        Self {
            id: peer.id,
            peer,
            executor,
            connected_nodes: HashMap::new(),

            analysis_node_id,
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

    pub fn executor_mut(&mut self) -> &mut Executor {
        &mut self.executor
    }

    // Assign keys to consensus nodes and send all consensus node public keys to each consensus node
    pub fn assign_key_to_consensus_node(&mut self) {
        let distribute_tbls_key_vec = generate_bls_keys(&self.connected_nodes, 1);

        println!("key count: {}", distribute_tbls_key_vec.len());
        let key = distribute_tbls_key_vec[0].tbls_key.clone();
        let key_msg = CommandMessage {
            command: Command::AssignTBLSKeypair(key),
        };
        let serialized_key = coder::serialize_into_bytes(&key_msg);
        println!("{:?}", serialized_key);
        let de_key: CommandMessage = coder::deserialize_for_bytes(&serialized_key);
        println!("{:#?}", de_key);

        let mut consensus_node_pks: HashMap<Vec<u8>, ConsensusNodePKInfo> = HashMap::new();
        for key_info in distribute_tbls_key_vec {
            let msg = CommandMessage {
                command: Command::AssignTBLSKeypair(key_info.tbls_key.clone()),
            };
            let serialized_msg = coder::serialize_into_bytes(&msg);
            self.peer_mut()
                .network_swarm_mut()
                .behaviour_mut()
                .unicast
                .send_message(&key_info.peer_id, serialized_msg);

            let db_key = key_info.peer_id.clone().to_bytes();
            let consensus_node_pk_info = ConsensusNodePKInfo {
                number: key_info.number,
                public_key: key_info.tbls_key.public_key,
            };
            let db_value = coder::serialize_into_bytes(&consensus_node_pk_info);
            self.executor.db.write(&db_key, &db_value);
            consensus_node_pks.insert(db_key, consensus_node_pk_info);
        }

        let msg = CommandMessage {
            command: Command::DistributeConsensusNodePKsInfo(consensus_node_pks),
        };
        let serialized_msg = coder::serialize_into_bytes(&msg);
        let topic = IdentTopic::new("Consensus");
        if let Err(e) = self
            .peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), serialized_msg)
        {
            eprintln!("Publish message error:{:?}", e);
        }
    }

    // Initiate a set of consensus requests
    pub fn make_consensus_requests(&mut self, count: usize) {
        let msg = generate_consensus_requests_command(count);
        println!("Request: {:#?}", msg);
        let serialized_msg = serialize_into_bytes(&msg);
        let topic = IdentTopic::new("Consensus");
        if let Err(e) = self
            .peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), serialized_msg)
        {
            eprintln!("Publish message error:{:?}", e);
        }
    }

    pub fn init(&mut self) {
        let id_bytes = self.id_bytes();
        let component = Component::Controller(id_bytes);
        let interactive_message = InteractiveMessage::ComponentInfo(component);
        let message = Message {
            message: interactive_message,
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
                                "AssignKey" => {
                                    self.assign_key_to_consensus_node();
                                }
                                "ProtocolStart" => {}
                                _ => {
                                    println!("Command is not found!");
                                }
                            },
                            Some(count) => {
                                self.make_consensus_requests(count);
                            }
                        }
                    }
                }
                event = self.peer.network_swarm_mut().select_next_some() => match event {
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        let msg: CommandMessage = coder::deserialize_for_bytes(&message.data);
                        match msg.command {
                            // MessageType::Reply(_) => {
                            //     self.executor.message_handler(&self.id.to_bytes(), &message.data).await;
                            // }
                            _ => {}
                        }
                    }
                    SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                        propagation_source: _peer_id,
                        message_id: _id,
                        message,
                    })) => {
                        self.executor.message_handler(&self.id.to_bytes(), &message.data).await;
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                        let swarm = self.peer.network_swarm_mut();
                        for (peer, _) in list {
                            println!("Discovered {:?}", &peer);
                            swarm.behaviour_mut().unicast.add_node_to_partial_view(&peer);
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                            // self.connected_nodes.insert(peer.to_string(), peer.clone());
                        }
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Expired(list))) => {
                        let swarm = self.peer.network_swarm_mut();
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
