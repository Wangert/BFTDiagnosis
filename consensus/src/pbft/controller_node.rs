use std::{collections::HashMap, error::Error};

use chrono::Local;
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
use tokio::io::{self, AsyncBufReadExt};
use utils::coder::{self, deserialize_for_bytes, serialize_into_bytes};

use crate::pbft::message::{MessageType, Request};

use super::{executor::ControllerExecutor, message::Message};

pub struct ControllerNode {
    pub id: PeerId,
    pub network_peer: Box<Peer>,
    pub executor: Box<ControllerExecutor>,
    pub connected_nodes: HashMap<String, PeerId>,
    pub current_leader: PeerId,
}

impl ControllerNode {
    pub fn new(peer: Box<Peer>, port: &str) -> ControllerNode {
        let db_path = format!("./storage/data/{}_public_keys", port);
        let executor = ControllerExecutor::new(&db_path);
        ControllerNode {
            id: peer.id,
            network_peer: peer,
            executor: Box::new(executor),
            connected_nodes: HashMap::new(),
            current_leader: PeerId::random(),
        }
    }

    pub async fn network_peer_start(&mut self) -> Result<(), Box<dyn Error>> {
        self.network_peer.swarm_start(false).await?;
        let topic = IdentTopic::new("DistributePK");
        self.network_peer.network_swarm_mut().behaviour_mut().gossipsub.subscribe(&topic);
        self.message_handler_start().await;

        Ok(())
    }

    pub async fn message_handler_start(&mut self) {
        let swarm = if let Some(s) = &mut self.network_peer.network.swarm {
            s
        } else {
            panic!("【network_peer】: Not build swarm")
        };

        let mut stdin = io::BufReader::new(io::stdin()).lines();

        loop {
            tokio::select! {
                line = stdin.next_line() => {
                    if let Ok(Some(command)) = line {
                        println!("{}", command);
                        if command.eq("key") {
                            let msg = Message { msg_type: MessageType::DistributePK };
                            let serialized_msg = coder::serialize_into_bytes(&msg);
                            let topic = IdentTopic::new("Consensus");
                            if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), serialized_msg) {
                                eprintln!("Publish message error:{:?}", e);
                            }
                        } else {
                            println!("Peer: {}", command);
                            let peer_id = self.connected_nodes.get(&command).unwrap().clone();

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
                            let dt = chrono::Local::now();
                            let timestamp: i64 = dt.timestamp_millis();
                            println!("发送时间为：{}",timestamp);
                            //self.executor.state.client_state = ClientState::Waiting;
                            //self.send_request("operation_test", &peer);
                        };
                    }
                }
                event = swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        let msg: Message = coder::deserialize_for_bytes(&message.data);
                        match msg.msg_type {
                            MessageType::Reply(_) => {
                                //println!("Reply Message: {:?} from peer: {}", &msg, peer.to_string());
                                self.executor.message_handler(&self.id.to_bytes(), &message.data).await;
                            }
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
}
