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
    state::State,
};

pub struct Executor {
    pub peer_id: String,
    pub state: State,
    pub local_logs: Box<LocalLogs>,
    pub broadcast_tx: Sender<String>,
    pub broadcast_rx: Receiver<String>,
    // pub message_tx: Sender<String>,
    // pub message_rx: Receiver<String>,
}

impl Executor {
    pub fn new(peer_id: &str) -> Executor {
        let (broadcast_tx, broadcast_rx) = mpsc::channel::<String>(5);
        //let (message_tx, message_rx) = mpsc::channel::<String>(5);

        Executor {
            peer_id: peer_id.to_string(),
            state: State::new(),
            local_logs: Box::new(LocalLogs::new()),
            broadcast_tx,
            broadcast_rx,
            // message_tx,
            // message_rx,
        }
    }

    pub async fn pbft_message_handler(&mut self, msg: &Vec<u8>) {
        //if let Some(msg) = self.message_rx.recv().await {
        //let msg = msg.as_bytes();
        let message: Message = coder::deserialize_for_bytes(msg);
        match message.msg_type {
            MessageType::Request(m) => {
                self.handle_request(&m).await;
            }
            MessageType::PrePrepare(m) => {
                self.handle_preprepare(&m);
            }
            MessageType::Prepare(m) => {
                self.handle_prepare(&m);
            }
            MessageType::Commit(m) => {
                self.handle_commit(&m);
            }
            MessageType::Reply(m) => {
                self.handle_reply(&m);
            } // _ => { eprintln!("Invalid pbft message!"); },
        }
    }

    pub async fn broadcast_preprepare(&self, msg: &str) {
        Peer::broadcast_message(&self.broadcast_tx, msg).await;
    }

    pub fn broadcast_prepare() {}
    pub fn broadcast_commit() {}
    pub fn reply() {}

    pub async fn handle_request(&self, r: &Request) {
        // verify request message(client signature)

        // generate number for request

        println!("handle_request ok!");

        // create PrePrepare message
        let view = self.state.view;
        let seq_number = self.state.current_seq_number + 1;

        let mut m_hash: Vec<u8> = vec![];
        let serialized_request = coder::serialize_into_bytes(&r);
        coder::get_hash(&serialized_request, &mut m_hash);
        // signature
        let signature = String::from("");

        let id = self.peer_id.clone();
        let preprepare = PrePrepare {
            view,
            number: seq_number,
            m_hash,
            m: serialized_request,
            peer_id: id,
            signature,
        };

        let broadcast_msg = Message {
            msg_type: MessageType::PrePrepare(preprepare),
        };

        let serialized_msg = coder::serialize_into_bytes(&broadcast_msg);
        let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

        // broadcast PrePrepare message
        self.broadcast_preprepare(str_msg).await;
    }

    pub fn handle_preprepare(&self, msg: &PrePrepare) {}
    pub fn handle_prepare(&self, msg: &Prepare) {}
    pub fn handle_commit(&self, msg: &Commit) {}
    pub fn handle_reply(&self, msg: &Reply) {}
}
