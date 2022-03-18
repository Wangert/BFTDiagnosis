use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    hash::{Hash, Hasher},
    sync::Arc,
    time::Duration,
};
use libp2p::{
    gossipsub::{
        Gossipsub, GossipsubConfigBuilder, GossipsubMessage, IdentTopic, MessageAuthenticity,
        MessageId,
    },
    identity::Keypair,
    Multiaddr, PeerId, Swarm,
};
use tokio::sync::Mutex;
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
    ) {
        let topic = IdentTopic::new("consensus");

        let swarm = {
            let message_id_fn = |message: &GossipsubMessage| {
                let mut s = DefaultHasher::new();
                let mut msg_hash_vec = if let Some(peer_id) = message.source {
                    peer_id.to_bytes()
                } else {
                    vec![]
                };

                let mut data = message.data.clone();
                msg_hash_vec.append(&mut data);
                msg_hash_vec.hash(&mut s);
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
    }

    pub async fn start_listen(
        &mut self,
        address: Multiaddr,
        peer_id: &PeerId,
        peer_keys: &Keypair,
        other_peers: Arc<Mutex<Vec<(Multiaddr, PeerId)>>>,
    ) -> Result<(), Box<dyn Error>> {
        tokio::time::sleep(Duration::from_secs(4)).await;

        //println!("[Gossipsub_Start]: {:?}", other_peers);
        let peers: Vec<(Multiaddr, PeerId)> = other_peers.lock().await.clone();

        // build gossipsub_swarm
        self.build_gossipsub_swarm(peer_id, peer_keys, &peers);

        let gossipsub_swarm = if let Some(s) = &mut self.swarm {
            s
        } else {
            panic!("【network_peer】: Not build gossipsub swarm")
        };

        // start gossipsub listen
        gossipsub_swarm.listen_on(address)?;

        for (addr, _) in peers {
            match gossipsub_swarm.dial(addr.clone()) {
                Ok(_) => println!("【Dialed {:?}】", addr),
                Err(e) => println!("Dial {:?} failed: {:?}", addr, e),
            }
        }

        Ok(())
    }
}
