use libp2p::{
    gossipsub::{Gossipsub, GossipsubEvent},
    mdns::{Mdns, MdnsEvent},
    NetworkBehaviour,
};
use crate::p2p_protocols::floodsub::behaviour::{Floodsub, FloodsubEvent};

use super::unicast::behaviour::{Unicast, UnicastEvent};

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
pub struct BaseBehaviour {
    pub unicast: Unicast,
    pub gossipsub: Gossipsub,
    pub floodsub: Floodsub,
    pub mdns: Mdns,
}

#[derive(Debug)]
pub enum OutEvent {
    Unicast(UnicastEvent),
    Gossipsub(GossipsubEvent),
    Floodsub(FloodsubEvent),
    Mdns(MdnsEvent),
}

impl From<UnicastEvent> for OutEvent {
    fn from(v: UnicastEvent) -> Self {
        Self::Unicast(v)
    }
}

impl From<MdnsEvent> for OutEvent {
    fn from(v: MdnsEvent) -> Self {
        Self::Mdns(v)
    }
}

impl From<FloodsubEvent> for OutEvent {
    fn from(v: FloodsubEvent) -> Self {
        Self::Floodsub(v)
    }
}

impl From<GossipsubEvent> for OutEvent {
    fn from(v: GossipsubEvent) -> Self {
        Self::Gossipsub(v)
    }
}

// impl NetworkBehaviourEventProcess<UnicastEvent> for BaseBehaviour {
//     fn inject_event(&mut self, event: UnicastEvent) {
//         match event {
//             UnicastEvent::Message(msg) => {
//                 let data = String::from_utf8_lossy(&msg.data);
//                 let sequence_number = String::from_utf8_lossy(&msg.sequence_number);
//                 let peer = String::from_utf8_lossy(&msg.source);
//                 println!("Unicast Message: {} with id: {} from peer: {}", data, sequence_number, peer) ;
//             }
//         }
//     }
// }

// impl NetworkBehaviourEventProcess<GossipsubEvent> for BaseBehaviour {
//     fn inject_event(&mut self, event: GossipsubEvent) {
//         match event {
//             GossipsubEvent::Message {
//                 propagation_source: peer_id,
//                 message_id: id,
//                 message,
//             } => {
//                 println!(
//                     "Got message: {} with id: {} from peer: {:?}",
//                     String::from_utf8_lossy(&message.data),
//                     id,
//                     peer_id
//                 );
//             }
//             _ => {}
//         }
//     }
// }

// impl NetworkBehaviourEventProcess<MdnsEvent> for BaseBehaviour {
//     fn inject_event(&mut self, event: MdnsEvent) {
//         match event {
//             MdnsEvent::Discovered(list) => {
//                 for (peer, _) in list {
//                     println!("Discovered {:?}", &peer);
//                     self.unicast.add_node_to_partial_view(&peer);
//                     self.gossipsub.add_explicit_peer(&peer);
//                 }
//             }
//             MdnsEvent::Expired(list) => {
//                 for (peer, _) in list {
//                     if !self.mdns.has_node(&peer) {
//                         self.unicast.remove_node_from_partial_view(&peer);
//                         self.gossipsub.remove_explicit_peer(&peer);
//                     }
//                 }
//             }
//         }
//     }
// }
