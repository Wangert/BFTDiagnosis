use std::{collections::HashMap, error::Error};

use libp2p::{
    futures::StreamExt,
    gossipsub::{GossipsubEvent, IdentTopic},
    mdns::MdnsEvent,
    swarm::SwarmEvent,
    PeerId,
};
use network::{
    p2p_protocols::{base_behaviour::OutEvent, unicast::behaviour::UnicastEvent},
    peer::Peer,
};

use utils::coder::{self};

use crate::chain_hotstuff::consensus_node::{message::MessageType, state::Mode};

use super::{common::get_request_hash, executor::Executor, message::Message};

pub struct Node {
    pub id: PeerId,
    pub network_peer: Box<Peer>,
    pub executor: Box<Executor>,
    pub request_buffer: Vec<String>,
    pub request_buffer_map: HashMap<String, (Vec<u8>, usize)>,
    pub connected_nodes: HashMap<String, PeerId>,
    pub current_leader: PeerId,
}

impl Node {
    pub fn new(peer: Box<Peer>, port: &str) -> Self {
        let db_path = format!("./storage/data/{}_public_keys", port);
        let executor = Executor::new(&db_path);
        Self {
            id: peer.id,
            network_peer: peer,
            executor: Box::new(executor),
            request_buffer: Vec::new(),
            request_buffer_map: HashMap::new(),
            connected_nodes: HashMap::new(),
            current_leader: PeerId::random(),
        }
    }

    pub async fn network_peer_start(&mut self) -> Result<(), Box<dyn Error>> {
        self.network_peer.swarm_start(true).await?;
        self.executor.proposal_state_check();
        self.message_handler_start().await;

        Ok(())
    }

    pub async fn message_handler_start(&mut self) {
        let swarm = if let Some(s) = &mut self.network_peer.network.swarm {
            s
        } else {
            panic!("【network_peer】: Not build swarm")
        };

        // Kick it off
        loop {
            tokio::select! {
                // start consensus notify
                _ = self.executor.consensus_notify.notified() => {
                    // get first request in buffer
                    let request_hash_option = self.request_buffer.first();
                    if let None = request_hash_option {
                        continue;
                    }
                    let request_hash = request_hash_option.unwrap();
                    let request_info = self.request_buffer_map.get(request_hash).unwrap();

                    // handle request
                    self.executor.message_handler(&self.id.to_bytes(), &request_info.0).await;
                },
                _ = self.executor.view_timeout_notify.notified() => {
                    let current_view_timeout = self.executor.state.current_view_timeout;
                    self.executor.state.current_view_timeout = current_view_timeout * 2;

                    // next view
                    self.executor.next_view(&self.id.to_bytes()).await;

                },
                Some(msg) = self.executor.msg_rx.recv() => {
                    //let serialized_msg = msg.clone().into_bytes();
                    let message: Message = coder::deserialize_for_bytes(&msg[..]);
                    match message.msg_type {
                        MessageType::Generic(generic) => {
                            if let None = generic.block {
                                let leader_id = PeerId::from_bytes(&self.executor.state.current_leader).expect("Leader peer id error.");
                                swarm.behaviour_mut().unicast.send_message(&leader_id, msg);
                            } else {
                                let topic = IdentTopic::new("Consensus");
                                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), msg) {
                                eprintln!("Publish message error:{:?}", e);
                                }
                            }
                        }
                        MessageType::Vote(_) => {
                            let leader_id = PeerId::from_bytes(&self.executor.state.next_leader).expect("Leader peer id error.");
                            swarm.behaviour_mut().unicast.send_message(&leader_id, msg);

                            self.executor.next_view(&self.id.to_bytes()).await;
                        }
                        MessageType::End(request_hash) => {
                            let request_info = self.request_buffer_map.get(&request_hash);
                            if let Some(_) = request_info {
                                self.request_buffer.remove(0);
                                self.request_buffer_map.remove(&request_hash);
                            }

                            let count = self.request_buffer.len();
                            if count == 0  {
                                *self.executor.state.mode.lock().await = Mode::Init;
                            } else {
                                *self.executor.state.mode.lock().await = Mode::NotIsLeader(count as u64);
                            }

                            println!("Current request buffer: {:?}", self.request_buffer);
                            println!("Current request buffer map: {:?}", self.request_buffer_map);
                        }
                        _ => {
                            let topic = IdentTopic::new("Consensus");
                            if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), msg) {
                                eprintln!("Publish message error:{:?}", e);
                            }
                        }
                    }

                },
                event = swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        // println!("tblskey.");
                        self.executor.message_handler(&self.id.to_bytes(), &message.data).await;
                    }
                    SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                        propagation_source: _peer_id,
                        message_id: _id,
                        message,
                    })) => {
                        let msg: Message = coder::deserialize_for_bytes(&message.data);
                        match msg.msg_type {
                            MessageType::Request(request) => {
                                let request_hash = get_request_hash(&request);
                                let count = self.request_buffer.len();
                                self.request_buffer.insert(count, request_hash.clone());
                                //self.request_buffer.push(request_hash.clone());
                                self.request_buffer_map.insert(request_hash, (message.data, count));

                                let mode_value = *self.executor.state.mode.lock().await;
                                match mode_value {
                                    Mode::Do(n) => {
                                        *self.executor.state.mode.lock().await = Mode::Do(n + 1);
                                    }
                                    Mode::Done(n) => {
                                        *self.executor.state.mode.lock().await = Mode::Done(n + 1);
                                    }
                                    Mode::NotIsLeader(n) => {
                                        *self.executor.state.mode.lock().await = Mode::NotIsLeader(n + 1);
                                    }
                                    Mode::Init => {
                                        *self.executor.state.mode.lock().await = Mode::NotIsLeader(1);
                                    }
                                }
                            }
                            _ => {
                                self.executor.message_handler(&self.id.to_bytes(), &message.data).await;
                            }
                        }
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                        for (peer, _) in list {
                            println!("Discovered {:?}", &peer);
                            swarm.behaviour_mut().unicast.add_node_to_partial_view(&peer);
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                            self.connected_nodes.insert(peer.to_string(), peer.clone());
                        }
                        //println!("Connected_nodes: {:?}", self.connected_nodes.lock().await);
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Expired(list))) => {
                        for (peer, _) in list {
                            if !swarm.behaviour_mut().mdns.has_node(&peer) {
                                swarm.behaviour_mut().unicast.remove_node_from_partial_view(&peer);
                                swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer);
                                self.connected_nodes.remove(&peer.to_string());
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
