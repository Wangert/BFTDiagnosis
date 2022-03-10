use std::sync::Arc;
use crate::{discovery::MdnsSwarm, gossipsub::GossipsubSwarm};
use futures::StreamExt;
use libp2p::{
    gossipsub::{GossipsubEvent, IdentTopic},
    identity::Keypair,
    swarm::SwarmEvent,
    Multiaddr, PeerId,
};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};

pub struct Peer {
    pub id: PeerId,
    pub mdns_addr: Multiaddr,
    pub gossipsub_addr: Multiaddr,
    pub addresses: Vec<String>,
    keys: Box<Keypair>,

    pub mdns_swarm: MdnsSwarm,
    //pub gossipsub_swarm: Arc<Mutex<GossipsubSwarm>>,
    pub gossipsub_swarm: GossipsubSwarm,

    // other peers' address and peerid
    pub other_peers_info: Box<Vec<(Multiaddr, PeerId)>>,
}

impl Peer {
    pub fn new(mdns_addr: Multiaddr, gossipsub_addr: Multiaddr) -> Peer {
        // Create a random PeerID
        let keys = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keys.public());

        Peer {
            id: peer_id,
            mdns_addr,
            gossipsub_addr,
            addresses: vec![],

            mdns_swarm: MdnsSwarm::new(&keys),
            gossipsub_swarm: GossipsubSwarm::new(&keys),

            keys: Box::new(keys),
            other_peers_info: Box::new(vec![]),
        }
    }

    pub fn get_keys(&self) -> Box<Keypair> {
        self.keys.clone()
    }

    // Set an address
    pub fn set_address(&mut self, addr: Multiaddr) {
        self.mdns_addr = addr;
    }

    pub fn set_gossipsub_addr(&mut self, addr: Multiaddr) {
        self.gossipsub_addr = addr;
    }

    pub async fn mdns_start(&mut self, other_peers: Arc<Mutex<Vec<(Multiaddr, PeerId)>>>) {
        let address = self.mdns_addr.clone();
        let peer_id = &self.id;

        let r = self.mdns_swarm.start(address, peer_id, other_peers).await;
        match r {
            Ok(_) => {}
            Err(e) => eprintln!("[mdns_swarm](start failed):{:?}", e),
        }
    }

    pub async fn gossipsub_start(&mut self, other_peers: Arc<Mutex<Vec<(Multiaddr, PeerId)>>>) {
        let address = self.gossipsub_addr.clone();
        let peer_id = &self.id;
        let peer_keys = &self.keys;

        let r = self
            .gossipsub_swarm
            .start_listen(address, peer_id, peer_keys, other_peers)
            .await;
        match r {
            Ok(_) => {}
            Err(e) => eprintln!("[gossipsub_swarm](start failed):{:?}", e),
        }
    }

    pub async fn message_handler_start(&mut self, rx: &mut Receiver<String>) {
        let swarm = if let Some(s) = &mut self.gossipsub_swarm.swarm {
            s
        } else {
            panic!("【network_peer】: Not build gossipsub swarm")
        };
        // Kick it off
        loop {
            tokio::select! {
                Some(msg) = rx.recv() => {
                    let topic = IdentTopic::new("consensus");
                    if let Err(e) = swarm.behaviour_mut().publish(topic.clone(), msg) {
                        eprintln!("Publish message error:{:?}", e);
                    }
                },
                event = swarm.select_next_some() => match event {
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

    pub async fn broadcast_message(tx: &Sender<String>, msg: &str) {
        tx.send(msg.to_string()).await.unwrap();
    }

    pub async fn run(&mut self, rx: &mut Receiver<String>) {
        let keys = self.keys.clone();
        let mut mdns_swarm = MdnsSwarm::new(&keys);

        let mdns_addr = self.mdns_addr.clone();
        let peer_id = self.id.clone();

        let other_peers: Arc<Mutex<Vec<(Multiaddr, PeerId)>>> = Arc::new(Mutex::new(vec![]));
        let g_peers = Arc::clone(&other_peers);

        tokio::spawn(async move {
            let _ = mdns_swarm.start(mdns_addr, &peer_id, other_peers).await;
        });

        self.gossipsub_start(g_peers).await;
        self.message_handler_start(rx).await;
    }
}
