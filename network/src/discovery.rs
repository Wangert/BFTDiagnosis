use std::{error::Error, sync::Arc};

use futures::StreamExt;
use libp2p::{mdns::{Mdns, MdnsConfig, MdnsEvent}, Swarm, PeerId, swarm::SwarmEvent, identity::Keypair, Multiaddr, multiaddr::Protocol};
use tokio::sync::Mutex;

use crate::{peer::Peer, transport::CMTTransport};

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
        // let behaviour_result = block_on(future);
        // let behaviour = match behaviour_result {
        //     Ok(b) => b,
        //     Err(e) => panic!("【network_peer】:{:?}", e),
        // };

        let transport = self.cmt_transport.0.clone();
        let op_swarm = Some(Box::new(Swarm::new(transport, behaviour, peer_id)));
        self.swarm = op_swarm;
        Ok(())
    }

    pub async fn start(&mut self, peer: &mut Peer, other_peers: Arc<Mutex<Vec<(Multiaddr, PeerId)>>>) -> Result<(), Box<dyn Error>> {
        self.build_swarm(peer.id).await?;

        let discovery_swarm = if let Some(s) = &mut self.swarm {
            s
        } else {
            panic!("【network_peer】: Not build discovery swarm.")
        };

        let addr = peer.address.clone();
        discovery_swarm.listen_on(addr)?;

        loop {
            match discovery_swarm.select_next_some().await {
                SwarmEvent::Behaviour(MdnsEvent::Discovered(peers)) => {
                    for (peer_id, addr) in peers {
                        println!("discovered {} {}", peer_id, addr);
                        //peer.other_peers_info.push((addr, peer_id));
                        let mut g_addr = addr.clone();
                        let g_port = if let Some(Protocol::Tcp(port)) = g_addr.pop() {
                            Protocol::Tcp(port+100)
                        } else {
                            Protocol::Tcp(0)
                        };

                        g_addr.push(g_port);
                        other_peers.lock().await.push((g_addr, peer_id));

                        println!("Other_peers_info: {:?}", other_peers);
                    }
                    
                    for l in discovery_swarm.listeners() {
                        println!("ex{:?}", l);
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
