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
    tcp::Tcp,
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

use super::binary_agreement::{ba_broadcast::BvBroadcast, binary_agreement::BinaryAgreement};
use super::reliable_broadcast::{
    broadcast::{Broadcast, PeerSet},
    message::{self, *},
};

pub struct Node {
    pub id: PeerId,
    pub network_peer: Box<Peer>,
    pub connected_nodes: HashMap<String, PeerId>,
    //pub rbc: Broadcast,
    //pub aba: BinaryAgreement,
    //pub ACS
}

impl Node {
    pub fn new(peer: Box<Peer>) -> Node {
        let my_id = peer.id.to_string();
        let connected_nodes = HashMap::new();
        let peer_set = PeerSet::new();
        // let rbc = Broadcast::new(my_id.as_str(), 1);
        // let netinfo = Network::new(my_id.as_str(),false,None,None,None,Vec::new());

        //let netinfo = Network::new(my_id.as_str(), is_consensus_node, sks, pks, );
        //let aba = BinaryAgreement::new(netinfo, 1);
        Node {
            id: peer.id,
            network_peer: peer,
            connected_nodes,
            // netinfo:netinfo,
            // rbc,
        }
    }

    pub async fn network_peer_start(
        &mut self,
        is_consensus_node: bool,
    ) -> Result<(), Box<dyn Error>> {
        println!("PeerID is :{}", self.id.to_string());
        self.network_peer.swarm_start(is_consensus_node).await?;

        self.client_message_handler_start().await;

        Ok(())
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
                    if let Ok(Some(command)) = line {

                        if command.eq("key") {
                            let distribute_tbls_key_vec = generate_bls_keys(&self.connected_nodes, 1);
                            println!("key count: {}", distribute_tbls_key_vec.len());

                            let key = distribute_tbls_key_vec[0].tbls_key.clone();
                            let key_msg = super::message::Message::TBLSKey(key);
                            let serialized_key = coder::serialize_into_bytes(&key_msg);
                            println!("{:?}", serialized_key);
                            let de_key: super::message::Message = coder::deserialize_for_bytes(&serialized_key);
                            println!("{:#?}", de_key);

                            let key1 = distribute_tbls_key_vec[1].tbls_key.clone();
                            let key_msg1 = super::message::Message::TBLSKey(key1);
                            let serialized_key1 = coder::serialize_into_bytes(&key_msg1);
                            println!("{:?}", serialized_key1);
                            let de_key1: super::message::Message = coder::deserialize_for_bytes(&serialized_key1);
                            println!("{:#?}", de_key1);

                            for key_info in distribute_tbls_key_vec {
                                let msg = super::message::Message::TBLSKey(key_info.tbls_key.clone());
                                let serialized_msg = coder::serialize_into_bytes(&msg);
                                swarm.behaviour_mut().unicast.send_message(&key_info.peer_id, serialized_msg);
                            }
                        }
                        else {
                        println!("command: {}", command);
                        if (command == "start") {
                            let request = super::message::Message::Request(super::message::Request{
                                data: command.to_string().into_bytes(),
                                client_id: self.id.to_string().into_bytes(),
                            });
                            let serialized_msg = serialize_into_bytes(&request);
                            for i in self.connected_nodes.keys() {
                                let peer_id = self.connected_nodes.get(i).unwrap();
                                swarm.behaviour_mut().unicast.send_message(&peer_id, serialized_msg.clone());
                            }


                        }
                        else {
                            println!("请重新输入！");
                        }

                        // let data = super::subset::message::Message{
                        //     proposer_id: self.id.to_string(),
                        //     content: request,
                        // };

                        //let deserialized_msg:super::subset::message::Message = deserialize_for_bytes(&serialized_msg[..]);

                        //println!("Client发送的Request请求: {:?}\n 目标节点为:{}", &deserialized_msg,peer_id.to_string());

                        // let serialized_msg = serialize_into_bytes(&Message::Request(request));
                        // let deserialized_msg: Message = deserialize_for_bytes(&serialized_msg[..]);
                        // let peer_id = self.connected_nodes.get(&command).unwrap();
                        // println!("Client发送的Request请求: {:?}\n 目标节点为:{}", &deserialized_msg,peer_id.to_string());
                        // swarm.behaviour_mut().unicast.send_message(&peer_id, serialized_msg.clone());
                        // for peer_id in self.connected_nodes.values_mut(){
                        //     println!("Client发送的Request请求: {:?}\n 目标节点为:{}", &deserialized_msg,peer_id.to_string());
                        //     swarm.behaviour_mut().unicast.send_message(&peer_id, serialized_msg.clone());
                        // }
                        }



                    }
                }
                event = swarm.select_next_some() => match event {

                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        let msg: Message = coder::deserialize_for_bytes(&message.data);
                        let peer = PeerId::from_bytes(&message.source).expect("Source peer error.");
                        match msg {
                            Message::Reply(_) => {
                                println!("Reply Message: {:?} from peer: {}", &msg, peer.to_string());
                                //self.rbc.handle_message(&peer.to_string(), &message.data).await;
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

    //Client发送消息
    // pub fn send_request(&mut self, operation: &str, target_node: &str) {
    //     let swarm = if let Some(s) = &mut self.network_peer.network.swarm {
    //         s
    //     } else {
    //         panic!("【network_peer】: Not build swarm")
    //     };

    //     let peer_id = self.connected_nodes.get(target_node).unwrap().clone();

    //     let request = message::Request {
    //         data: String::from(operation),
    //         client_id: self.id.clone().to_bytes(),
    //     };

    //     let serialized_msg = serialize_into_bytes(&Message::Request(request));
    //     let send_msg = std::str::from_utf8(&serialized_msg).unwrap();

    //     let deserialized_msg: Message = deserialize_for_bytes(send_msg.as_bytes());

    //     println!("Deserialzed_msg: {:?}", &deserialized_msg);
    //     swarm
    //         .behaviour_mut()
    //         .unicast
    //         .send_message(&peer_id, send_msg);
    // }
}
