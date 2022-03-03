use std::{error::Error, sync::Arc, thread, time::Duration};

use async_std::sync::Mutex;
use futures::{join, executor::block_on, future::join};
use libp2p::{
    identity::Keypair,
    Multiaddr, PeerId, multiaddr::Protocol,
};

use crate::{discovery::MdnsSwarm, gossipsub::GossipsubSwarm};




pub struct Peer {
    pub id: PeerId,
    pub address: Multiaddr,
    pub gossipsub_port: u16,
    pub addresses: Vec<String>,
    keys: Box<Keypair>,

    // other peers' address and peerid
    pub other_peers_info: Box<Vec<(Multiaddr, PeerId)>>,
}

impl Peer {
    pub fn new(address: Multiaddr, g_port: u16) -> Peer {
        // Create a random PeerID
        let keys = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keys.public());

        Peer {
            id: peer_id,
            address,
            gossipsub_port: g_port,
            addresses: vec![],
            keys: Box::new(keys),
            other_peers_info: Box::new(vec![]),
        }
    }

    pub fn get_keys(&self) -> Box<Keypair> {
        self.keys.clone()
    }

    // Set an address
    pub fn set_address(&mut self, addresses: &Vec<String>) {
        self.addresses = addresses.to_vec();
    }

    // pub async fn run(&mut self) -> (Result<(), Box<dyn Error>>, Result<(), Box<dyn Error>>) {
    //     let addrs: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
    //     // let new_ip = Ipv4Addr::new(127, 0, 0, 1);
    //     // addrs.push(Protocol::Ip4(new_ip));
    //     let discovery_future = self.discovery_start(addrs.clone());
    //     // let discovery_start_result = block_on(discovery_future);
    //     // match discovery_start_result {
    //     //     Ok(_) => println!("peer runing successful"),
    //     //     Err(e) => panic!("{:?}", e),
    //     // }

    //     self.build_gossipsub_swarm();

    //     let gossipsub_future = self.gossipsub_start(addrs.clone());
    //     join!(discovery_future, gossipsub_future)
    // }
}

pub async fn run(peer: &mut Peer, mdns_swarm: &mut MdnsSwarm, gossipsub_swarm: &mut GossipsubSwarm) -> (Result<(), Box<dyn Error>>, Result<(), Box<dyn Error>>) {
    
    let keys = peer.keys.clone();
    //let addr = peer.address.clone();
    let peer_id = peer.id.clone();

    let addr = "/ip4/10.162.182.172/tcp/51103".parse().unwrap();
    
    let other_peers: Arc<Mutex<Vec<(Multiaddr, PeerId)>>> = Arc::new(Mutex::new(vec![]));
    //let d_peers = Arc::clone(&other_peers);
    let g_peers = Arc::clone(&other_peers);


    let discovery_future = mdns_swarm.start(peer, other_peers);

    //thread::sleep(Duration::from_secs(5));
    //let other_peers = peer.other_peers_info.clone();
    //println!("Run Other peers: {:?}", other_peers);

    // let discovery_start_result = block_on(discovery_future);
    // match discovery_start_result {
    //     Ok(_) => println!("Peer runing successful!"),
    //     Err(e) => panic!("{:?}", e),
    // }

    //let mut gossipsub_swarm = GossipsubSwarm::new(&keys);
    
    let gossipsub_future = gossipsub_swarm.gossipsub_start(addr, &peer_id, &keys, g_peers);

    // let r1 = discovery_future.await;
    // let r2 = gossipsub_future.await;

    join!(discovery_future, gossipsub_future)
    // (r1, r2)
}
