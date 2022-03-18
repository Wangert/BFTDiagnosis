use network::peer::Peer;
use tokio::sync::mpsc::{self, Receiver, Sender};
use utils::coder::{self, get_hash_str};

use crate::pbft::common::get_message_key;

use super::{
    local_logs::LocalLogs,
    message::{Commit, Message, MessageType, PrePrepare, Prepare, Reply, Request},
    state::State,
};

// Node consensus executor
pub struct Executor {
    pub state: State,
    pub local_logs: Box<LocalLogs>,
    pub broadcast_tx: Sender<String>,
    pub broadcast_rx: Receiver<String>,
}

impl Executor {
    pub fn new() -> Executor {
        let (broadcast_tx, broadcast_rx) = mpsc::channel::<String>(10);

        Executor {
            state: State::new(),
            local_logs: Box::new(LocalLogs::new()),
            broadcast_tx,
            broadcast_rx,
        }
    }

    pub async fn pbft_message_handler(&mut self, source: &str, msg: &Vec<u8>) {
        //if let Some(msg) = self.message_rx.recv().await {
        //let msg = msg.as_bytes();
        let message: Message = coder::deserialize_for_bytes(msg);
        match message.msg_type {
            MessageType::Request(m) => {
                self.handle_request(&m).await;
            }
            MessageType::PrePrepare(m) => {
                self.handle_preprepare(source, &m).await;
            }
            MessageType::Prepare(m) => {
                self.handle_prepare(source, &m).await;
            }
            MessageType::Commit(m) => {
                self.handle_commit(source, &m).await;
            }
            MessageType::Reply(m) => {
                self.handle_reply(&m);
            } // _ => { eprintln!("Invalid pbft message!"); },
        }
    }

    pub async fn broadcast_preprepare(&self, msg: &str) {
        Peer::broadcast_message(&self.broadcast_tx, msg).await;
    }

    pub async fn broadcast_prepare(&self, msg: &str) {
        Peer::broadcast_message(&self.broadcast_tx, msg).await;
    }

    pub async fn broadcast_commit(&self, msg: &str) {
        Peer::broadcast_message(&self.broadcast_tx, msg).await;
    }

    pub fn reply() {}

    pub async fn handle_request(&mut self, r: &Request) {
        // verify request message(client signature)

        // generate number for request

        println!("handle_request ok!");

        // create PrePrepare message
        let view = self.state.view;
        let seq_number = self.state.current_seq_number + 1;

        let serialized_request = coder::serialize_into_bytes(&r);
        let m_hash = get_hash_str(&serialized_request);
        // signature
        let signature = String::from("");

        //let id = self.peer_id.clone();
        let preprepare = PrePrepare {
            view,
            number: seq_number,
            m_hash,
            m: serialized_request,
            signature,
            from_peer_id: String::from(""),
        };

        let broadcast_msg = Message {
            msg_type: MessageType::PrePrepare(preprepare),
        };

        let serialized_msg = coder::serialize_into_bytes(&broadcast_msg);
        let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

        // broadcast PrePrepare message
        self.broadcast_preprepare(str_msg).await;
    }

    pub async fn handle_preprepare(&mut self, source: &str, msg: &PrePrepare) {
        // verify preprepare message

        // record request message
        let serialized_request = msg.m.clone();
        let m: Request = coder::deserialize_for_bytes(&serialized_request);
        self.local_logs
            .record_message_handler(MessageType::Request(m));
        let request = self.local_logs.get_local_messages_by_hash(&msg.m_hash);
        println!("###################Current Request Messages#################");
        println!("{:?}", &request);
        println!("###############################################################");

        // record preprepare message
        let mut record_msg = msg.clone();
        record_msg.from_peer_id = source.to_string();
        self.local_logs
            .record_message_handler(MessageType::PrePrepare(record_msg));
        let key_str = get_message_key(MessageType::PrePrepare(msg.clone()));

        let msg_vec = self.local_logs.get_local_messages_by_hash(&key_str);
        println!("###################Current PrePrepare Messages#################");
        println!("{:?}", &msg_vec);
        println!("###############################################################");

        // create prepare message
        let view = self.state.view;
        let seq_number = msg.number;
        let m_hash = msg.m_hash.clone();
        let signature = String::from("");
        let prepare = Prepare {
            view,
            number: seq_number,
            m_hash,
            from_peer_id: String::from(""),
            signature,
        };

        let broadcast_msg = Message {
            msg_type: MessageType::Prepare(prepare),
        };

        let serialized_msg = coder::serialize_into_bytes(&broadcast_msg);
        let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

        // broadcast prepare message
        self.broadcast_prepare(str_msg).await;
    }

    pub async fn handle_prepare(&mut self, source: &str, msg: &Prepare) {
        // verify prepare message

        // record message
        let mut record_msg = msg.clone();
        record_msg.from_peer_id = source.to_string();
        self.local_logs
            .record_message_handler(MessageType::Prepare(record_msg));

        let key_str = get_message_key(MessageType::Prepare(msg.clone()));
        let msg_vec = self.local_logs.get_local_messages_by_hash(&key_str);
        println!("###################Current Prepare Messages#################");
        println!("{:?}", &msg_vec);
        println!("###############################################################");

        // create commit message
        let view = self.state.view;
        let seq_number = msg.number;
        let m_hash = msg.m_hash.clone();
        let signature = String::from("");
        let commit = Commit {
            view,
            number: seq_number,
            m_hash,
            from_peer_id: String::from(""),
            signature,
        };

        let broadcast_msg = Message {
            msg_type: MessageType::Commit(commit),
        };

        let serialized_msg = coder::serialize_into_bytes(&broadcast_msg);
        let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

        // broadcast commit message
        self.broadcast_commit(str_msg).await;
    }

    pub async fn handle_commit(&mut self, source: &str, msg: &Commit) {
        // verify commit message

        // record message
        let mut record_msg = msg.clone();
        record_msg.from_peer_id = source.to_string();
        self.local_logs
            .record_message_handler(MessageType::Commit(record_msg));

        let key_str = get_message_key(MessageType::Commit(msg.clone()));
        let msg_vec = self.local_logs.get_local_messages_by_hash(&key_str);
        println!("###################Current Commit Messages#################");
        println!("{:?}", &msg_vec);
        println!("###############################################################");

        // create reply message
        let view = self.state.view;
        let seq_number = msg.number;
        let m_hash = msg.m_hash.clone();
        let signature = String::from("");

        let mut client_addr = "".to_string();
        let mut timestamp = "".to_string();
        match self.local_logs.get_local_messages_by_hash(&m_hash).get(0) {
            Some(MessageType::Request(m)) => {
                client_addr = m.client_addr.clone();
                timestamp = m.timestamp.clone();
            }
            _ => {
                eprintln!("Not have local request record! ")
            }
        };

        let reply = Reply {
            client_addr,
            timestamp,
            number: seq_number,
            from_peer_id: String::from(""),
            signature,
            result: "ok!".as_bytes().to_vec(),
            view,
        };

        let broadcast_msg = Message {
            msg_type: MessageType::Reply(reply),
        };

        let serialized_msg = coder::serialize_into_bytes(&broadcast_msg);
        let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

        // broadcast PrePrepare message
        self.broadcast_commit(str_msg).await;
    }

    pub fn handle_reply(&self, msg: &Reply) {}
}
