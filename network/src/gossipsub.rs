use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    hash::{Hash, Hasher},
    time::Duration, sync::Arc, thread,
};

use async_std::{io::{self, prelude::BufReadExt}, sync::Mutex};
use futures::{select, StreamExt};
use libp2p::{
    gossipsub::{
        Gossipsub, GossipsubConfigBuilder, GossipsubEvent, GossipsubMessage, IdentTopic,
        MessageAuthenticity, MessageId,
    },
    identity::Keypair,
    swarm::SwarmEvent,
    Multiaddr, PeerId, Swarm,
};

use crate::transport::CMTTransport;

pub struct GossipsubSwarm {
    pub cmt_transport: Box<CMTTransport>,
    pub swarm: Option<Box<Swarm<Gossipsub>>>,
}

impl GossipsubSwarm {
    pub fn new(peer_keys: &Keypair) -> GossipsubSwarm {
        let transport = Box::new(CMTTransport::new(&peer_keys));
        GossipsubSwarm {
            cmt_transport: transport,
            swarm: None,
        }
    }

    pub fn build_gossipsub_swarm(
        &mut self,
        peer_id: &PeerId,
        peer_keys: &Keypair,
        other_peers: &Vec<(Multiaddr, PeerId)>,
    ) -> IdentTopic {
        let topic = IdentTopic::new("consensus");

        let swarm = {
            let message_id_fn = |message: &GossipsubMessage| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                MessageId::from(s.finish().to_string())
            };

            let gossipsub_config = GossipsubConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(5))
                .message_id_fn(message_id_fn)
                .build()
                .expect("Valid config");

            let keypair = peer_keys.clone();
            let mut gossipsub =
                Gossipsub::new(MessageAuthenticity::Signed(keypair), gossipsub_config)
                    .expect("Gossipsub correct configuration");

            gossipsub.subscribe(&topic).unwrap();

            println!("Build Other peers:{:?}", other_peers);


            //let peers = other_peers.clone();
            for (_, peer_id) in other_peers {
                gossipsub.add_explicit_peer(peer_id);
            }

            let transport = self.cmt_transport.0.clone();
            Swarm::new(transport, gossipsub, *peer_id)
        };

        let op_swarm = Some(Box::new(swarm));
        self.swarm = op_swarm;

        topic
    }

    pub async fn gossipsub_start(
        &mut self,
        address: Multiaddr,
        peer_id: &PeerId,
        peer_keys: &Keypair,
        other_peers: Arc<Mutex<Vec<(Multiaddr, PeerId)>>>,
    ) -> Result<(), Box<dyn Error>> {
        //thread::sleep(Duration::from_secs(5));
        async_std::task::sleep(Duration::from_secs(5)).await;

        println!("[Gossipsub_Start]: {:?}", other_peers);
        let peers: Vec<(Multiaddr, PeerId)> = other_peers.lock().await.clone();
        let topic = self.build_gossipsub_swarm(peer_id, peer_keys, &peers);
        let gossipsub_swarm = if let Some(s) = &mut self.swarm {
            s
        } else {
            panic!("【network_peer】: Not build gossipsub swarm")
        };

        gossipsub_swarm.listen_on(address)?;

        //let peers_2 = Arc::clone(&other_peers);
        // connect other peer
        for (addr, _) in peers {
            println!("Other Multiaddr:{:?}", addr);
            match gossipsub_swarm.dial(addr.clone()) {
                Ok(_) => println!("Dialed {:?}", addr),
                Err(e) => println!("Dial {:?} failed: {:?}", addr, e),
            }
        }

        // Read full lines from stdin
        let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();

        //let topic = IdentTopic::new("consensus");

        // Kick it off
        loop {
            select! {
                line = stdin.select_next_some() => {
                    //println!("{:?}", gossipsub_swarm.behaviour());
                    
                    if let Err(e) = gossipsub_swarm
                        .behaviour_mut()
                        .publish(topic.clone(), line.expect("Stdin not to close").as_bytes())
                    {
                        println!("Publish error: {:?}", e);
                    }
                },
                event = gossipsub_swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(GossipsubEvent::Message {
                        propagation_source: peer_id,
                        message_id: id,
                        message,
                    }) => println!(
                        "Got message: {} with id: {} from peer: {:?}",
                        String::from_utf8_lossy(&message.data),
                        id,
                        peer_id
                    ),
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on {:?}", address);
                    }
                    _ => {}
                }
            }
        }
    }
}
