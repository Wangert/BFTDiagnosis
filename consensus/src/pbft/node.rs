use std::{collections::HashMap, error::Error, sync::Arc};

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
use tokio::{io::AsyncReadExt, net::TcpStream, sync::Mutex};
use utils::coder;

use crate::pbft::message::MessageType;

use super::{executor::Executor, message::Message};

pub struct Node {
    pub id: PeerId,
    pub network_peer: Box<Peer>,
    pub executor: Box<Executor>,
    pub connected_nodes: Arc<Mutex<HashMap<String, PeerId>>>,
    pub current_leader: Arc<Mutex<Vec<PeerId>>>,
}

impl Node {
    pub fn new(peer: Box<Peer>) -> Node {
        let executor = Executor::new();
        Node {
            id: peer.id,
            network_peer: peer,
            executor: Box::new(executor),
            connected_nodes: Arc::new(Mutex::new(HashMap::new())),
            current_leader: Arc::new(Mutex::new(vec![PeerId::random()])),
        }
    }

    pub async fn network_peer_start(&mut self) -> Result<(), Box<dyn Error>> {
        self.network_peer.swarm_start().await?;
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
                Some(msg) = self.executor.broadcast_rx.recv() => {
                    let serialized_msg = msg.clone().into_bytes();
                    let message: Message = coder::deserialize_for_bytes(&serialized_msg);
                    match message.msg_type {
                        MessageType::Request(_) => {
                            let leaders = self.current_leader.lock().await.clone();
                            swarm.behaviour_mut().unicast.send_message(&leaders[0], msg);
                            println!("Send message to {}", &leaders[0].to_string());
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
                        println!("Unicast Message: {:?} from peer: {}", &msg, peer.to_string());

                        self.executor.pbft_message_handler(&peer.to_string(), &message.data).await;
                    }
                    SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                        propagation_source: peer_id,
                        message_id: id,
                        message,
                    })) => {
                        println!("Got message: {} with id: {} from peer: {:?}", String::from_utf8_lossy(&message.data), id, &peer_id);
                        let msg: Message = coder::deserialize_for_bytes(&message.data);
                        println!("Gossip Deserialized Message: {:?}", &msg);

                        // let str_msg: String = String::from_utf8_lossy(&message.data).parse().unwrap();
                        // if let Err(e) = self.proto.message_tx.send(str_msg).await {
                        //     eprintln!("Send message_tx error:{:?}", e);
                        // };

                        self.executor.pbft_message_handler(&peer_id.to_string(), &message.data).await;
                        //self.pbft_message_handler(std::str::from_utf8(&message.data).unwrap());
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                        for (peer, _) in list {
                            println!("Discovered {:?}", &peer);
                            swarm.behaviour_mut().unicast.add_node_to_partial_view(&peer);
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                            self.connected_nodes.lock().await.insert(peer.to_string(), peer.clone());
                        }

                        //println!("Connected_nodes: {:?}", self.connected_nodes.lock().await);
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Expired(list))) => {
                        for (peer, _) in list {
                            if !swarm.behaviour_mut().mdns.has_node(&peer) {
                                swarm.behaviour_mut().unicast.remove_node_from_partial_view(&peer);
                                swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer);
                                self.connected_nodes.lock().await.remove(&peer.to_string());
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
