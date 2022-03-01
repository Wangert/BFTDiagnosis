use futures::executor::block_on;
use libp2p::{
    identity::Keypair,
    mdns::{Mdns, MdnsConfig},
    PeerId, Swarm,
};

use crate::transport::CMTTransport;

pub struct Peer {
    pub id: String,
    pub addresses: Vec<String>,
    keys: Box<Keypair>,
    pub discovery_swarm: Box<Swarm<Mdns>>,
}

impl Peer {
    pub fn new() -> Peer {
        // Create a random PeerID
        let keys = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keys.public());
        let transport = CMTTransport::new(&keys);

        let future = Mdns::new(MdnsConfig::default());
        let behaviour_result = block_on(future);
        let behaviour = match behaviour_result {
            Ok(b) => b,
            Err(e) => panic!("【network_peer】:{:?}", e),
        };

        let discovery_swarm = Box::new(Swarm::new(transport.0, behaviour, peer_id));

        Peer {
            id: peer_id.to_string(),
            addresses: vec![],
            keys: Box::new(keys),
            discovery_swarm,
        }
    }

    // Set an address
    pub fn set_address(&mut self, addresses: &Vec<String>) {
        self.addresses = addresses.to_vec();
    }

    pub fn get_keys(&self) -> Box<Keypair> {
        self.keys.clone()
    }
}
