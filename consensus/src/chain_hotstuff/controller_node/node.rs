use std::{collections::HashMap, error::Error};

use libp2p::{
    futures::StreamExt,
    gossipsub::{GossipsubEvent, IdentTopic},
    mdns::MdnsEvent,
    swarm::SwarmEvent,
    PeerId,
};
use network::{
    p2p_protocols::{base_behaviour::OutEvent, unicast::behaviour::UnicastEvent, floodsub::{topic::Topic, behaviour::FloodsubEvent}},
    peer::Peer,
};

use tokio::io::{self, AsyncBufReadExt};
use utils::coder::{self, serialize_into_bytes};

use crate::chain_hotstuff::controller_node::{
    common::{create_requests, generate_bls_keys},
    message::{ConsensusNodePKInfo, Message, MessageType},
};

use super::executor::Executor;

pub struct Node {
    pub id: PeerId,
    pub network_peer: Box<Peer>,
    pub executor: Box<Executor>,
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
            connected_nodes: HashMap::new(),
            current_leader: PeerId::random(),
        }
    }

    pub fn peer(&self) -> PeerId {
        self.id
    }

    pub async fn network_peer_start(&mut self) -> Result<(), Box<dyn Error>> {
        self.network_peer.swarm_start(false).await?;
        let peer = self.peer();

        let topic = Topic::new("Consensus");
        self.network_peer.network_swarm_mut().behaviour_mut().floodsub.subscribe(topic);
       
        self.network_peer.network_swarm_mut().behaviour_mut().floodsub.add_node_to_partial_view(peer);

        self.message_handler_start().await;

        
        Ok(())
    }

    pub async fn message_handler_start(&mut self) {
        let swarm = if let Some(s) = &mut self.network_peer.network.swarm {
            s
        } else {
            panic!("【network_peer】: Not build swarm")
        };

        let peer = self.id.clone();
        

        let mut stdin = io::BufReader::new(io::stdin()).lines();

        loop {
            tokio::select! {
                line = stdin.next_line() => {
                    if let Ok(Some(command)) = line {
                        println!("{}", command);
                        if command.eq("key") {
                            let distribute_tbls_key_vec = generate_bls_keys(&self.connected_nodes, self.executor.state.fault_tolerance_count);

                            println!("key count: {}", distribute_tbls_key_vec.len());
                            let key = distribute_tbls_key_vec[0].tbls_key.clone();
                            let key_msg = Message { msg_type: MessageType::TBLSKey(key)};
                            let serialized_key = coder::serialize_into_bytes(&key_msg);
                            println!("{:?}", serialized_key);
                            let de_key: Message = coder::deserialize_for_bytes(&serialized_key);
                            println!("{:#?}", de_key);

                            let mut consensus_node_pks: HashMap<Vec<u8>, ConsensusNodePKInfo> = HashMap::new();
                            for key_info in distribute_tbls_key_vec {
                                let msg = Message { msg_type: MessageType::TBLSKey(key_info.tbls_key.clone()) };
                                let serialized_msg = coder::serialize_into_bytes(&msg);
                                swarm.behaviour_mut().unicast.send_message(&key_info.peer_id, serialized_msg);

                                let db_key = key_info.peer_id.clone().to_bytes();
                                let consensus_node_pk_info = ConsensusNodePKInfo {
                                    number: key_info.number,
                                    public_key: key_info.tbls_key.public_key,
                                };
                                let db_value = coder::serialize_into_bytes(&consensus_node_pk_info);
                                self.executor.db.write(&db_key, &db_value);
                                consensus_node_pks.insert(db_key, consensus_node_pk_info);
                            }

                            let msg = Message { msg_type: MessageType::ConsensusNodePKsInfo(consensus_node_pks) };
                            let serialized_msg = coder::serialize_into_bytes(&msg);
                            // let topic = IdentTopic::new("Consensus");
                            // if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), serialized_msg) {
                            //     eprintln!("Publish message error:{:?}", e);
                            // }
                            
                            let topic = Topic::new("Consensus");
                            swarm.behaviour_mut().floodsub.publish(topic,serialized_msg);
                        } else {
                            
                            let count = command.parse::<usize>().unwrap();
                            let msg_vec = create_requests(count);

                            for msg in msg_vec {
                                let dt = chrono::Local::now();
                                let timestamp: i64 = dt.timestamp_millis();
                                println!("Request: {:?}，发送时间为：{}", msg,timestamp); 
                                let serialized_msg = serialize_into_bytes(&msg);
                                // let topic = IdentTopic::new("Consensus");
                                // if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), serialized_msg) {
                                //     eprintln!("Publish message error:{:?}", e);
                                // }
                                let topic = Topic::new("Consensus");
                                swarm.behaviour_mut().floodsub.publish(topic,serialized_msg);
                            }
                        };
                    }
                }
                event = swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        let msg: Message = coder::deserialize_for_bytes(&message.data);
                        match msg.msg_type {
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
                    SwarmEvent::Behaviour(OutEvent::Floodsub(
                        FloodsubEvent::Message(message)
                    )) => {
                        self.executor.message_handler(&self.id.to_bytes(), &message.data).await;
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                        for (peer, _) in list {
                            println!("Discovered {:?}", &peer);
                            swarm.behaviour_mut().unicast.add_node_to_partial_view(&peer);
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                            swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer.clone());
                            self.connected_nodes.insert(peer.to_string(), peer.clone());
                        }
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Expired(list))) => {
                        for (peer, _) in list {
                            if !swarm.behaviour_mut().mdns.has_node(&peer) {
                                swarm.behaviour_mut().unicast.remove_node_from_partial_view(&peer);
                                swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer);
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
