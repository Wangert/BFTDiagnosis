use crate::{
    base_swarm::BaseSwarm,
    p2p_protocols::{base_behaviour::OutEvent, unicast::behaviour::UnicastEvent},
};

use futures::StreamExt;
use libp2p::{
    gossipsub::{GossipsubEvent, IdentTopic},
    identity::Keypair,
    swarm::SwarmEvent,
    Multiaddr, PeerId, mdns::MdnsEvent,
};
use std::{error::Error, sync::Arc};
use tokio::{
    io::{self, AsyncBufReadExt},
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
};

pub struct Peer {
    // peer id
    pub id: PeerId,
    // swarm listen address
    pub swarm_addr: Multiaddr,
    // peer keypair
    keypair: Keypair,
    // network swarm
    pub network: BaseSwarm,
}

impl Peer {
    pub fn new(swarm_addr: Multiaddr) -> Peer {
        // Create a random PeerID
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());

        Peer {
            id: peer_id.clone(),
            swarm_addr,
            network: BaseSwarm::new(),
            keypair,
        }
    }

    pub fn get_keys(&self) -> Keypair {
        self.keypair.clone()
    }

    pub async fn swarm_start(&mut self, is_consensus_node: bool) -> Result<(), Box<dyn Error>> {
        self.network
            .build(self.id.clone(), self.keypair.clone(), is_consensus_node)
            .await?;
        self.network.start(self.swarm_addr.clone())?;

        Ok(())
    }

    pub async fn message_handler_start(&mut self) -> Result<(), Box<dyn Error>> {
        let swarm = if let Some(swarm) = &mut self.network.swarm {
            swarm
        } else {
            panic!("【network_peer】: Not build swarm")
        };

        let mut stdin = io::BufReader::new(io::stdin()).lines();

        loop {
            tokio::select! {
                line = stdin.next_line() => {
                    let line = line?.expect("stdin closed.");
                    println!("Line: {}", line);

                    if line.contains("send") {
                        swarm.behaviour_mut().unicast.rand_send_message(line);
                    } else if line.contains("publish") {
                        let topic = IdentTopic::new("consensus");
                        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, line) {
                            eprintln!("Publish message error:{:?}", e);
                        };
                    }
                }
                event = swarm.select_next_some() => match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on {:?}", address);
                    }
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(msg))) => {
                        let data = String::from_utf8_lossy(&msg.data);
                        let sequence_number = String::from_utf8_lossy(&msg.sequence_number);
                        let peer = String::from_utf8_lossy(&msg.source);
                        println!("Unicast Message: {} with id: {} from peer: {}", data, sequence_number, peer) ;
                    }
                    SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                        propagation_source: peer_id,
                        message_id: id,
                        message,})) => {
                                println!(
                                    "Got message: {} with id: {} from peer: {:?}",
                                    String::from_utf8_lossy(&message.data),
                                    id,
                                    peer_id
                                );
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                        for (peer, _) in list {
                            println!("Discovered {:?}", &peer);
                            swarm.behaviour_mut().unicast.add_node_to_partial_view(&peer);
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                        }
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Expired(list))) => {
                        for (peer, _) in list {
                            if !swarm.behaviour_mut().mdns.has_node(&peer) {
                                swarm.behaviour_mut().unicast.remove_node_from_partial_view(&peer);
                                swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // pub async fn mdns_start(&mut self, other_peers: Arc<Mutex<Vec<(Multiaddr, PeerId)>>>) {
    //     let address = self.mdns_addr.clone();
    //     let peer_id = &self.id;

    //     let r = self.mdns_swarm.start(address, peer_id, other_peers).await;
    //     match r {
    //         Ok(_) => {}
    //         Err(e) => eprintln!("[mdns_swarm](start failed):{:?}", e),
    //     }
    // }

    // pub async fn gossipsub_start(&mut self, other_peers: Arc<Mutex<Vec<(Multiaddr, PeerId)>>>) {
    //     let address = self.gossipsub_addr.clone();
    //     let peer_id = &self.id;
    //     let peer_keys = &self.keys;

    //     let r = self
    //         .gossipsub_swarm
    //         .start_listen(address, peer_id, peer_keys, other_peers)
    //         .await;
    //     match r {
    //         Ok(_) => {}
    //         Err(e) => eprintln!("[gossipsub_swarm](start failed):{:?}", e),
    //     }
    // }

    // pub async fn message_handler_start(&mut self, rx: &mut Receiver<String>) {
    //     let swarm = if let Some(s) = &mut self.gossipsub_swarm.swarm {
    //         s
    //     } else {
    //         panic!("【network_peer】: Not build gossipsub swarm")
    //     };
    //     // Kick it off
    //     loop {
    //         tokio::select! {
    //             Some(msg) = rx.recv() => {
    //                 let topic = IdentTopic::new("consensus");
    //                 if let Err(e) = swarm.behaviour_mut().publish(topic.clone(), msg) {
    //                     eprintln!("Publish message error:{:?}", e);
    //                 }
    //             },
    //             event = swarm.select_next_some() => match event {
    //                 SwarmEvent::Behaviour(GossipsubEvent::Message {
    //                     propagation_source: peer_id,
    //                     message_id: id,
    //                     message,
    //                 }) => {
    //                     println!("Got message: {} with id: {} from peer: {:?}", String::from_utf8_lossy(&message.data), id, peer_id);
    //                 }
    //                 SwarmEvent::NewListenAddr { address, .. } => {
    //                     println!("Listening on {:?}", address);
    //                 }
    //                 _ => {}
    //             }
    //         }
    //     }
    // }

    // pub async fn broadcast_message(tx: &Sender<String>, msg: &str) {
    //     tx.send(msg.to_string()).await.unwrap();
    // }

    // pub async fn run(&mut self) {
    //     let keys = self.keys.clone();
    //     let mut mdns_swarm = MdnsSwarm::new(&keys);

    //     let mdns_addr = self.mdns_addr.clone();
    //     let peer_id = self.id.clone();

    //     let other_peers: Arc<Mutex<Vec<(Multiaddr, PeerId)>>> = Arc::new(Mutex::new(vec![]));
    //     let g_peers = Arc::clone(&other_peers);

    //     tokio::spawn(async move {
    //         let _ = mdns_swarm.start(mdns_addr, &peer_id, other_peers).await;
    //     });

    //     self.gossipsub_start(g_peers).await;
    //     //self.message_handler_start(rx).await;
    // }
}
