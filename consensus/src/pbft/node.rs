use std::{collections::HashMap, error::Error, sync::Arc};

use chrono::Local;
use libp2p::{
    futures::{future::ok, StreamExt},
    gossipsub::{GossipsubEvent, IdentTopic},
    mdns::MdnsEvent,
    swarm::SwarmEvent,
    PeerId,
};
use network::{
    p2p_protocols::{base_behaviour::OutEvent, unicast::behaviour::UnicastEvent},
    peer::Peer,
    tcp::Tcp,
};
use tokio::{io::{AsyncReadExt, self, AsyncBufReadExt}, net::TcpStream, sync::Mutex};
use utils::coder::{self, serialize_into_bytes, deserialize_for_bytes};

use crate::pbft::{message::{MessageType, Request}, state::ClientState};

use super::{executor::Executor, message::Message};

pub struct Node {
    pub id: PeerId,
    pub network_peer: Box<Peer>,
    pub executor: Box<Executor>,
    pub connected_nodes: HashMap<String, PeerId>,
    pub current_leader: PeerId,
}

impl Node {
    pub fn new(peer: Box<Peer>) -> Node {
        let executor = Executor::new();
        Node {
            id: peer.id,
            network_peer: peer,
            executor: Box::new(executor),
            connected_nodes: HashMap::new(),
            current_leader: PeerId::random(),
        }
    }

    pub async fn network_peer_start(
        &mut self,
        is_consensus_node: bool,
    ) -> Result<(), Box<dyn Error>> {
        self.network_peer.swarm_start(is_consensus_node).await?;
        if is_consensus_node {
            self.executor.timeout_check_start();
            self.message_handler_start().await;
        } else {
            self.client_message_handler_start().await;
        }
        

        Ok(())
    }

    pub fn send_request(&mut self, operation: &str, target_node: &str) {
        let swarm = if let Some(s) = &mut self.network_peer.network.swarm {
            s
        } else {
            panic!("【network_peer】: Not build swarm")
        };

        let peer_id = self.connected_nodes.get(target_node).unwrap().clone();

        let request = Request {
            operation: String::from(operation),
            client_id: self.id.clone().to_bytes(),
            timestamp: Local::now().timestamp().to_string(),
            signature: String::from("signature"),
        };
    
        let msg = Message {
            msg_type: MessageType::Request(request),
        };
    
        let serialized_msg = serialize_into_bytes(&msg);
        let send_msg = std::str::from_utf8(&serialized_msg).unwrap();

        let deserialized_msg: Message = deserialize_for_bytes(send_msg.as_bytes());

        println!("Deserialzed_msg: {:?}", &deserialized_msg);
        swarm.behaviour_mut().unicast.send_message(&peer_id, send_msg);
    }

    pub async fn client_message_handler_start(&mut self) {
        let swarm = if let Some(s) = &mut self.network_peer.network.swarm {
            s
        } else {
            panic!("【network_peer】: Not build swarm")
        };

        let mut stdin = io::BufReader::new(io::stdin()).lines();

        loop {
            tokio::select! {
                line = stdin.next_line() => {
                    if let Ok(Some(peer)) = line {
                        println!("Peer: {}", peer);

                        let peer_id = self.connected_nodes.get(&peer).unwrap().clone();

                        let request = Request {
                            operation: String::from("operation"),
                            client_id: self.id.clone().to_bytes(),
                            timestamp: Local::now().timestamp().to_string(),
                            signature: String::from("signature"),
                        };
                    
                        let msg = Message {
                            msg_type: MessageType::Request(request),
                        };
                    
                        let serialized_msg = serialize_into_bytes(&msg);
                        //let send_msg = std::str::from_utf8(&serialized_msg).unwrap();
                
                        let deserialized_msg: Message = deserialize_for_bytes(&serialized_msg[..]);

                        println!("Deserialzed_msg: {:?}", &deserialized_msg);
                        swarm.behaviour_mut().unicast.send_message(&peer_id, serialized_msg);
                        self.executor.state.client_state = ClientState::Waiting;
                        //self.send_request("operation_test", &peer);
                    }
                }
                event = swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        let msg: Message = coder::deserialize_for_bytes(&message.data);
                        let peer = PeerId::from_bytes(&message.source).expect("Source peer error.");
                        match msg.msg_type {
                            MessageType::Reply(_) => {
                                //println!("Reply Message: {:?} from peer: {}", &msg, peer.to_string());
                                self.executor.pbft_message_handler(&peer.to_string(), &message.data).await;
                            }
                            _ => {}
                        }
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                        for (peer, _) in list {
                            println!("Discovered {:?}", &peer);
                            swarm.behaviour_mut().unicast.add_node_to_partial_view(&peer);
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                            self.connected_nodes.insert(peer.to_string(), peer.clone());
                        }
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

    pub async fn message_handler_start(&mut self) {
        let swarm = if let Some(s) = &mut self.network_peer.network.swarm {
            s
        } else {
            panic!("【network_peer】: Not build swarm")
        };

        // Kick it off
        loop {
            tokio::select! {
                _ = self.executor.viewchange_notify.notified() => {
                    self.executor.broadcast_viewchange().await;
                },
                Some(msg) = self.executor.msg_rx.recv() => {
                    //let serialized_msg = msg.clone().into_bytes();
                    let message: Message = coder::deserialize_for_bytes(&msg[..]);
                    match message.msg_type {
                        // MessageType::Request(_) => {
                        //     let leaders = self.current_leader.lock().await.clone();
                        //     swarm.behaviour_mut().unicast.send_message(&leaders[0], msg);
                        //     println!("Send message to {}", &leaders[0].to_string());
                        // }
                        MessageType::Reply(r) => {
                            let client_id = PeerId::from_bytes(&r.client_id).expect("client id error.");
                            swarm.behaviour_mut().unicast.send_message(&client_id, msg);
                        }
                        _ => {
                            let topic = IdentTopic::new("consensus");
                            if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), msg) {
                                eprintln!("Publish message error:{:?}", e);
                            }
                        }
                    }

                },
                event = swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        let msg: Message = coder::deserialize_for_bytes(&message.data);
                        let peer = PeerId::from_bytes(&message.source).expect("Source peer error.");
                        match msg.msg_type {
                            MessageType::Request(_) => {
                                println!("Client Request Message: {:?} from client: {}", &msg, peer.to_string());
                                self.executor.pbft_message_handler(&peer.to_string(), &message.data).await;
                            }
                            _ => {}
                        }
                    }
                    SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                        propagation_source: peer_id,
                        message_id: _id,
                        message,
                    })) => {
                        //println!("Got message: {} with id: {} from peer: {:?}", String::from_utf8_lossy(&message.data), id, &peer_id);
                        //let msg: Message = coder::deserialize_for_bytes(&message.data);
                        self.executor.pbft_message_handler(&peer_id.to_string(), &message.data).await;
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

#[cfg(test)]
mod node_tests {
    #[test]
    fn handle_request_works() {}
}
