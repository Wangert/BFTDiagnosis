use std::{error::Error, sync::Arc};

use futures::StreamExt;
use libp2p::{
    identity::Keypair,
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    multiaddr::Protocol,
    swarm::SwarmEvent,
    Multiaddr, PeerId, Swarm,
};
use tokio::sync::Mutex;

use crate::transport::CMTTransport;

pub struct MdnsSwarm {
    pub cmt_transport: Box<CMTTransport>,
    pub swarm: Option<Box<Swarm<Mdns>>>,
}

impl MdnsSwarm {
    pub fn new(peer_keys: &Keypair) -> MdnsSwarm {
        let transport = Box::new(CMTTransport::new(peer_keys));
        MdnsSwarm {
            cmt_transport: transport,
            swarm: None,
        }
    }

    // build a mDNS swarm for peer
    pub async fn build_swarm(&mut self, peer_id: PeerId) -> Result<(), Box<dyn Error>> {
        let behaviour = Mdns::new(MdnsConfig::default()).await?;

        let transport = self.cmt_transport.0.clone();
        let op_swarm = Some(Box::new(Swarm::new(transport, behaviour, peer_id)));

        self.swarm = op_swarm;

        Ok(())
    }

    pub async fn start(
        &mut self,
        address: Multiaddr,
        peer_id: &PeerId,
        other_peers: Arc<Mutex<Vec<(Multiaddr, PeerId)>>>,
    ) -> Result<(), Box<dyn Error>> {
        // build mdns swarm
        self.build_swarm(*peer_id).await?;

        let discovery_swarm = if let Some(s) = &mut self.swarm {
            s
        } else {
            panic!("【network_peer】: Not build discovery swarm.")
        };

        // start mdns listen
        discovery_swarm.listen_on(address)?;

        loop {
            match discovery_swarm.select_next_some().await {
                SwarmEvent::Behaviour(MdnsEvent::Discovered(peers)) => {
                    for l in discovery_swarm.listeners() {
                        //println!("===========================================================");
                        println!("【Mdns listening on: {:?}】", l);
                    }

                    for (peer_id, addr) in peers {
                        //println!("===========================================================");
                        println!("【Discovered:{} {}】", peer_id, addr);
                        //peer.other_peers_info.push((addr, peer_id));
                        let mut g_addr = addr.clone();
                        let g_port = if let Some(Protocol::Tcp(port)) = g_addr.pop() {
                            Protocol::Tcp(port + 100)
                        } else {
                            Protocol::Tcp(0)
                        };

                        g_addr.push(g_port);
                        other_peers.lock().await.push((g_addr, peer_id));

                        println!("========================【Other_peers_info】=============================");
                        println!("{:?}", other_peers);
                    }
                }
                SwarmEvent::Behaviour(MdnsEvent::Expired(expired)) => {
                    for (peer, addr) in expired {
                        println!("expired {} {}", peer, addr);
                    }
                }
                _ => {}
            }
        }
    }
}
