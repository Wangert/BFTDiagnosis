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
    message::{Command, ConsensusNodePKInfo, CommandMessage, ConsensusStartData, ConsensusEndData, ConsensusDataMessage, ConsensusData},
};

use super::executor::Executor;

pub struct AnalysisNode {
    pub id: PeerId,
    pub peer: Peer,
    pub connected_nodes: HashMap<String, PeerId>,

    controller_node_id: PeerId,
}

impl AnalysisNode {
    pub fn new(peer: Peer, controller_node_id: PeerId, port: &str) -> Self {
        // let db_path = format!("./storage/data/{}_public_keys", port);
        Self {
            id: peer.id,
            peer,
            connected_nodes: HashMap::new(),
            controller_node_id,
        }
    }

    pub async fn peer_start(&mut self) -> Result<(), Box<dyn Error>> {
        self.peer_mut().swarm_start(false).await?;
        self.message_handler_start().await;

        Ok(())
    }

    pub fn peer_mut(&mut self) -> &mut Peer {
        &mut self.peer
    }

    pub async fn message_handler_start(&mut self) {

        loop {
            tokio::select! {
                event = self.peer_mut().network_swarm_mut().select_next_some() => match event {
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        let peer_id = PeerId::from_bytes(&message.source[..]).unwrap();
                        if self.controller_node_id.to_string().eq(&peer_id.to_string()) {
                            let consensus_start_data: ConsensusStartData = coder::deserialize_for_bytes(&message.data);
                            println!("【ConsensusStartData(from {:?})】: {:?}", &peer_id.to_string(), &consensus_start_data);
                        } else if self.connected_nodes.contains_key(&peer_id.to_string()) {
                            let consensus_data_message: ConsensusDataMessage = coder::deserialize_for_bytes(&message.data);
                            match consensus_data_message.data {
                                ConsensusData::ConsensusStartData(data) => {
                                    println!("【ConsensusStartData(from {:?})】: {:?}", &peer_id.to_string(), &data);
                                }
                                ConsensusData::ConsensusEndData(data) => {
                                    println!("【ConsensusEndData(from {:?})】: {:?}", &peer_id.to_string(), &data);
                                }
                            };
                            
                            // let msg: Message = coder::deserialize_for_bytes(&message.data);
                            // match msg.command {
                            //     // MessageType::Reply(_) => {
                            //     //     self.executor.message_handler(&self.id.to_bytes(), &message.data).await;
                            //     // }
                            //     _ => {}
                            // }
                        }
                    }
                    SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                        propagation_source: _peer_id,
                        message_id: _id,
                        message,
                    })) => {
                        
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
