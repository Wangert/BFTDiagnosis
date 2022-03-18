use libp2p::{
    futures::StreamExt,
    gossipsub::{GossipsubEvent, IdentTopic},
    swarm::SwarmEvent,
};
use network::peer::Peer;
use tokio::sync::mpsc::{self, Receiver, Sender};
use utils::coder;

use super::{
    local_logs::LocalLogs,
    message::{Commit, Message, MessageType, PrePrepare, Prepare, Reply, Request},
    state::State, executor::Executor,
};

pub struct Node {
    pub id: String,
    pub network_peer: Box<Peer>,
    pub executor: Box<Executor>,
}

impl Node {
    pub fn new(peer: Box<Peer>) -> Node {
        let executor = Executor::new();
        Node {
            id: peer.id.to_string(),
            network_peer: peer,
            executor: Box::new(executor),
        }
    }

    pub async fn network_peer_start(&mut self) {
        self.network_peer.run().await;
        self.message_handler_start().await;
    }

    pub async fn message_handler_start(&mut self) {
        let swarm = if let Some(s) = &mut self.network_peer.gossipsub_swarm.swarm {
            s
        } else {
            panic!("【network_peer】: Not build gossipsub swarm")
        };
        // Kick it off
        loop {
            tokio::select! {
                Some(msg) = self.executor.broadcast_rx.recv() => {
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
                    }) => {
                        println!("Got message: {} with id: {} from peer: {:?}", String::from_utf8_lossy(&message.data), id, &peer_id);
                        let msg: Message = coder::deserialize_for_bytes(&message.data);
                        println!("Gossip Deserialized Message: {:?}", &msg);

                        // let str_msg: String = String::from_utf8_lossy(&message.data).parse().unwrap();
                        // if let Err(e) = self.proto.message_tx.send(str_msg).await {
                        //     eprintln!("Send message_tx error:{:?}", e);
                        // };

                        self.executor.pbft_message_handler(&peer_id.to_string(), &message.data).await;
                        //self.pbft_message_handler(std::str::from_utf8(&message.data).unwrap());
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on {:?}", address);
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod node_tests {
    #[test]
    fn handle_request_works() {}
}
