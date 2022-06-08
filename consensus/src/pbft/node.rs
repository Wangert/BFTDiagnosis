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

use utils::coder::{self, deserialize_for_bytes, serialize_into_bytes};

use crate::pbft::{
    message::{MessageType, Request},
};

use super::{executor::Executor, message::Message};

pub struct ConsensusNode {
    pub id: PeerId,
    pub network_peer: Box<Peer>,
    pub executor: Box<Executor>,
    pub connected_nodes: HashMap<String, PeerId>,
    pub current_leader: PeerId,
}

impl ConsensusNode {
    pub fn new(peer: Box<Peer>, port: &str) -> Self {
        let db_path = format!("./storage/data/{}_public_keys", port);
        let executor = Executor::new(&db_path);
        Self {
            id: peer.id,
            network_peer: peer,
            executor: Box::new(executor),
            connected_nodes: HashMap::new(),
            current_leader: PeerId::random(),
        }
    }

    pub async fn network_peer_start(&mut self) -> Result<(), Box<dyn Error>> {
        self.network_peer.swarm_start(true).await?;
        self.executor.timeout_check_start();
        self.message_handler_start().await;

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
        swarm
            .behaviour_mut()
            .unicast
            .send_message(&peer_id, send_msg);
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
                    self.executor.broadcast_viewchange(&self.id.to_bytes()).await;
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
                        MessageType::PublicKey(_) => {
                            let topic = IdentTopic::new("DistributePK");
                            if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), msg) {
                                eprintln!("Publish message error:{:?}", e);
                            }
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

                        //println!("Unicast Message is {:#?}", &msg);

                        match msg.msg_type {
                            // MessageType::PublicKey(pk) => {
                            //     let db_key = peer.to_base58().into_bytes();
                            //     println!("peer_id len: {}", db_key.len());
                            //     let db_value = pk.0;
                            //     self.executor.db.write(&db_key[..], &db_value[..]);

                            //     let pk = self.executor.db.read(&db_key[..]).unwrap();
                            //     println!("pkpkpkpkpkpkpkpkpkpkpkpkpkkpkpkpkkpkpkppkpkpk");
                            //     println!("{:?}'s public key is {:?}", &db_key, &pk);
                            // }
                            MessageType::Request(_) => {
                                println!("Client Request Message: {:?} from client: {}", &msg, peer.to_string());
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
