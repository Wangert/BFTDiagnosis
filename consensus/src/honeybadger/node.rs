use super::network::Network;
use crate::honeybadger::common::generate_bls_keys;
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
};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    sync::Arc,
};
use tokio::{
    io::{self, AsyncBufReadExt, AsyncReadExt},
    net::TcpStream,
    sync::Mutex,
};
use utils::coder::{self, deserialize_for_bytes, serialize_into_bytes};

//use crate::pbft::message::{MessageType, Request};
use super::binary_agreement::{ba_broadcast::BvBroadcast, binary_agreement::BinaryAgreement};
use super::reliable_broadcast::{
    broadcast::{Broadcast, PeerSet},
    message::{self, *},
};
use super::subset::subset::Subset;

use crate::honeybadger::subset::message::Message as SubsetMessage;
use crate::honeybadger::subset::message::MessageContent as SubsetMessageContent;

pub struct Node {
    pub id: PeerId,
    pub network_peer: Box<Peer>,
    pub connected_nodes: HashMap<String, PeerId>,
    //pub rbc: Broadcast,
    pub netinfo: Network,
    pub acs: Subset,
    //pub aba: BinaryAgreement,
    //pub ACS
}

impl Node {
    pub fn new(peer: Box<Peer>) -> Node {
        let my_id = peer.id.to_string();
        let connected_nodes = HashMap::new();
        let peer_set = PeerSet::new();
        //let rbc = Broadcast::new(my_id.as_str(), 1);
        let netinfo = Network::new(my_id.as_str(), true, None, None, None, Vec::new());
        let acs = Subset::new(netinfo.clone(), 1);
        //let netinfo = Network::new(my_id.as_str(), is_consensus_node, sks, pks, );
        //let aba = BinaryAgreement::new(netinfo, 1);
        Node {
            id: peer.id,
            network_peer: peer,
            connected_nodes,
            netinfo,
            acs,
            //rbc,
        }
    }

    pub async fn network_peer_start(
        &mut self,
        is_consensus_node: bool,
    ) -> Result<(), Box<dyn Error>> {
        println!("PeerID is :{}", self.id.to_string());
        self.network_peer.swarm_start(is_consensus_node).await?;
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
                                        Some(msg) = self.acs.rbc_msg_rx.recv() => {
                                            //let serialized_msg = msg.clone().into_bytes();
                                            //println!("rbc_msg_rx:{:?}",msg);
                                            let message: SubsetMessage = coder::deserialize_for_bytes(&msg[..]);
                                            match message.content {
                                                // Message::Request(_) => {
                                                //     let leaders = self.current_leader.lock().await.clone();
                                                //     swarm.behaviour_mut().unicast.send_message(&leaders[0], msg);
                                                //     println!("Send message to {}", &leaders[0].to_string());
                                                // }



                                                _ => {

                                                    //println!("{:?}",message);
                                                    let topic = IdentTopic::new("consensus");
                                                    if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), msg) {
                                                        eprintln!("Publish message error:{:?},{:?}", e,message);
                                                    }
                                                }
                                            }
                                        },

                                        Some(msg) = self.acs.aba_msg_rx.recv() => {
                                            let message: SubsetMessage = coder::deserialize_for_bytes(&msg[..]);
                                            match message.content {

                                                 SubsetMessageContent::Reply(proposer_id, result) => {
                                                        // self.acs.result.entry(proposer_id).or_
                                                        // println!("节点最终的共识结果为：{:?}",self.acs.result);
                                                     },
                                                    _ => {
                                                    let topic = IdentTopic::new("consensus");
                                                        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), msg) {
                                            }
                                        }
                                        }
                                            //println!("aba_msg_rx:{:?}",msg);

                                        },

                                        event = swarm.select_next_some() => match event {
                                            //共识节点收到单播（Client发送的Request请求）
                                            SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                                                println!("test");
                                                let msg: super::message::Message = coder::deserialize_for_bytes(&message.data);
                                                let peer = PeerId::from_bytes(&message.source).expect("Source peer error.");
                                                let control_id = PeerId::from_bytes(&message.source).expect("Source peer error.").to_string();

                                                match msg {
                                                    crate::honeybadger::message::Message::TBLSKey(key) => {
                                                        let mut peer_vec = Vec::new();
                                                        for key in self.connected_nodes.keys() {
                                                            if(key.to_string().as_bytes() != &message.source){
                                                                peer_vec.push(key.to_string());
                                                            }
                                                        }
                                                        peer_vec.push(self.id.to_string());
                                                        println!("{:?}",peer_vec);
                                                        let netinfo = Network::new(self.id.to_string().as_str(),true,Some(key.secret_key.0),Some(key.public_key), Some(key.pk_set),peer_vec);
                                                        self.netinfo = netinfo;
                                                        self.acs = Subset::new(self.netinfo.clone(), 1);

                                                    },
                                                    super::message::Message::Request(request) => {
                                                        //let data = request.data;
                                                        let data = self.id.to_string().as_bytes().to_vec();
                                                        self.acs.propose(data,control_id.as_str()).await;
                                                    },
                        }
                                            }
                                            SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                                                propagation_source: peer_id,
                                                message_id: id,
                                                message,
                                            })) => {
                                                let msg: SubsetMessage = coder::deserialize_for_bytes(&message.data);
                                                self.acs.handle_message(&peer_id.to_string(), &message.data).await;

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
