use crate::{
    base_swarm::BaseSwarm,
    p2p_protocols::{
        base_behaviour::{BaseBehaviour, OutEvent},
        unicast::behaviour::UnicastEvent,
    },
};

use futures::StreamExt;
use libp2p::{
    gossipsub::{GossipsubEvent, IdentTopic},
    identity::Keypair,
    mdns::MdnsEvent,
    swarm::SwarmEvent,
    Multiaddr, PeerId, Swarm,
};
use std::error::Error;
use tokio::io::{self, AsyncBufReadExt};

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

    pub fn network_mut(&mut self) -> &mut BaseSwarm {
        &mut self.network
    }

    pub fn network_swarm_mut(&mut self) -> &mut Swarm<BaseBehaviour> {
        self.network.swarm.as_mut().expect("Not build swarm!")
    }

    pub async fn swarm_start(&mut self, is_consensus_node: bool) -> Result<(), Box<dyn Error>> {
        self.network
            .build(self.id.clone(), self.keypair.clone(), is_consensus_node)
            .await?;
        self.network.start(self.swarm_addr.clone())?;

        Ok(())
    }

    // pub async fn message_handler_start(&mut self) -> Result<(), Box<dyn Error>> {
    //     let swarm = if let Some(swarm) = &mut self.network.swarm {
    //         swarm
    //     } else {
    //         panic!("【network_peer】: Not build swarm")
    //     };

    //     let mut stdin = io::BufReader::new(io::stdin()).lines();

    //     loop {
    //         tokio::select! {
    //             line = stdin.next_line() => {
    //                 let line = line?.expect("stdin closed.");
    //                 println!("Line: {}", line);

    //                 if line.contains("send") {
    //                     swarm.behaviour_mut().unicast.rand_send_message(line);
    //                 } else if line.contains("publish") {
    //                     let topic = IdentTopic::new("Consensus");
    //                     if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, line) {
    //                         eprintln!("Publish message error:{:?}", e);
    //                     };
    //                 }
    //             }
    //             event = swarm.select_next_some() => match event {
    //                 SwarmEvent::NewListenAddr { address, .. } => {
    //                     println!("\nListening on {:?}", address);
    //                 }
    //                 SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(msg))) => {
    //                     let data = String::from_utf8_lossy(&msg.data);
    //                     let sequence_number = String::from_utf8_lossy(&msg.sequence_number);
    //                     let peer = String::from_utf8_lossy(&msg.source);
    //                     println!("Unicast Message: {} with id: {} from peer: {}", data, sequence_number, peer) ;
    //                 }
    //                 SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
    //                     propagation_source: peer_id,
    //                     message_id: id,
    //                     message,})) => {
    //                             println!(
    //                                 "Got message: {} with id: {} from peer: {:?}",
    //                                 String::from_utf8_lossy(&message.data),
    //                                 id,
    //                                 peer_id
    //                             );
    //                 }
    //                 SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
    //                     for (peer, _) in list {
    //                         println!("\nDiscovered {:?}", &peer);
    //                         swarm.behaviour_mut().unicast.add_node_to_partial_view(&peer);
    //                         swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
    //                     }
    //                 }
    //                 SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Expired(list))) => {
    //                     for (peer, _) in list {
    //                         if !swarm.behaviour_mut().mdns.has_node(&peer) {
    //                             swarm.behaviour_mut().unicast.remove_node_from_partial_view(&peer);
    //                             swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer);
    //                         }
    //                     }
    //                 }
    //                 _ => {}
    //             }
    //         }
    //     }
    // }
}
