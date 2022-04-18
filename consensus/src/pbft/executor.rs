use tokio::sync::mpsc::{self, Receiver, Sender};
use utils::coder::{self, get_hash_str};

use crate::pbft::common::{get_message_key, WATER_LEVEL_DIFFERENCE};

use super::{
    local_logs::LocalLogs,
    message::{Commit, Message, MessageType, PrePrepare, Prepare, Reply, Request},
    state::State,
};

// Node consensus executor
pub struct Executor {
    pub state: State,
    pub local_logs: Box<LocalLogs>,
    pub msg_tx: Sender<Vec<u8>>,
    pub msg_rx: Receiver<Vec<u8>>,
}

impl Executor {
    pub fn new() -> Executor {
        let (msg_tx, msg_rx) = mpsc::channel::<Vec<u8>>(10);

        Executor {
            state: State::new(),
            local_logs: Box::new(LocalLogs::new()),
            msg_tx,
            msg_rx,
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
                self.handle_reply(source, &m);
            } // _ => { eprintln!("Invalid pbft message!"); },
        }
    }

    pub async fn broadcast_preprepare(&self, msg: &[u8]) {
        self.msg_tx.send(msg.to_vec()).await.unwrap();
    }

    pub async fn broadcast_prepare(&self, msg: &[u8]) {
        self.msg_tx.send(msg.to_vec()).await.unwrap();
    }

    pub async fn broadcast_commit(&self, msg: &[u8]) {
        self.msg_tx.send(msg.to_vec()).await.unwrap();
    }

    pub async fn reply(&self, msg: &[u8]) {
        self.msg_tx.send(msg.to_vec()).await.unwrap();
    }

    // handle request message
    pub async fn handle_request(&mut self, r: &Request) {
        // verify request message(client signature)

        // generate number for request

        println!("handle_request ok!");

        // create PrePrepare message
        let view = self.state.view;
        let seq_number = self.state.current_seq_number + 1;

        let serialized_request = coder::serialize_into_bytes(&r);
        self.local_logs.record_request(seq_number, &r.clone());
        let request = self
            .local_logs
            .get_local_request_by_sequence_number(seq_number);
        println!("###################Current Request Messages#################");
        println!("{:?}", &request);
        println!("###############################################################");

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
        //let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

        // broadcast PrePrepare message
        self.broadcast_preprepare(&serialized_msg[..]).await;
    }

    // handle preprepare message
    pub async fn handle_preprepare(&mut self, source: &str, msg: &PrePrepare) {
        // verify preprepare message
        // check sequence number
        let low = self.state.current_seq_number;
        let high = self.state.current_seq_number + WATER_LEVEL_DIFFERENCE;
        if msg.number <= low || msg.number > high {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return;
        }
        // check view
        if self.state.view != msg.view {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return;
        }

        // record request message
        let serialized_request = msg.m.clone();
        let m: Request = coder::deserialize_for_bytes(&serialized_request[..]);
        self.local_logs.record_request(msg.number, &m);
        let request = self
            .local_logs
            .get_local_request_by_sequence_number(msg.number);
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
        //let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

        // broadcast prepare message
        self.broadcast_prepare(&serialized_msg[..]).await;
    }

    pub async fn handle_prepare(&mut self, source: &str, msg: &Prepare) {
        if self.state.current_state == 1 {
            return;
        }
        // verify prepare message
        // check sequence number
        let low = self.state.current_seq_number;
        let high = self.state.current_seq_number + WATER_LEVEL_DIFFERENCE;
        if msg.number <= low || msg.number > high {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return;
        }
        // check view
        if self.state.view != msg.view {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return;
        }

        // check m hash
        let m = self
            .local_logs
            .get_local_request_by_sequence_number(msg.number);
        if let Some(m) = m {
            let request_hash = get_message_key(MessageType::Request(m.clone()));
            if request_hash.ne(&msg.m_hash) {
                eprintln!("Request hash error!");
                return;
            }
        } else {
            eprintln!("Request is not exsit!");
            return;
        }

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

        // check 2f+1 prepare messages (include current node)
        let current_count = self.local_logs.get_local_messages_count_by_hash(&key_str);
        let threshold = 2 * self.state.fault_tolerance_count;
        if current_count as u64 == threshold {
            self.state.current_state = 1;
            println!("【Prepare message to 2f+1, send commit message】");
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
            //let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

            // broadcast commit message
            self.broadcast_commit(&serialized_msg[..]).await;
        }
    }

    pub async fn handle_commit(&mut self, source: &str, msg: &Commit) {
        if self.state.current_state == 2 {
            return;
        }
        // verify commit message
        // check sequence number
        let low = self.state.current_seq_number;
        let high = self.state.current_seq_number + WATER_LEVEL_DIFFERENCE;
        if msg.number <= low || msg.number > high {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return;
        }
        // check view
        if self.state.view != msg.view {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return;
        }

        // check m hash
        let m = self
            .local_logs
            .get_local_request_by_sequence_number(msg.number);
        let m = if let Some(m) = m {
            m.clone()
        } else {
            panic!("Not have local request record!");
        };
        
        // check m hash
        let request_hash = get_message_key(MessageType::Request(m.clone()));
        if request_hash.ne(&msg.m_hash) {
            eprintln!("Request hash error!");
            return;
        }

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

        // check 2f+1 commit
        let current_count = self.local_logs.get_local_messages_count_by_hash(&key_str);
        let threshold = 2 * self.state.fault_tolerance_count;

        if current_count as u64 == threshold {
            self.state.current_state = 2;
            println!("【Commit message to 2f+1, send reply message】");
            // create reply message
            let view = self.state.view;
            let seq_number = msg.number;
            let signature = String::from("");

            let client_id = m.client_id;
            let timestamp = m.timestamp;

            let reply = Reply {
                client_id,
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
            //let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

            // Send Reply message to client
            self.reply(&serialized_msg[..]).await;
        }
    }

    pub fn handle_reply(&mut self, source: &str, msg: &Reply) {
        if self.state.current_state == 3 {
            return;
        }
        // verify signature

        // record reply 
        let mut record_msg = msg.clone();
        record_msg.from_peer_id = source.to_string();

        self.local_logs.record_message_handler(MessageType::Reply(record_msg));
        let key_str = get_message_key(MessageType::Reply(msg.clone()));
        let msg_vec = self.local_logs.get_local_messages_by_hash(&key_str);
        println!("###################Current Reply Messages#################");
        println!("{:?}", &msg_vec);
        println!("###############################################################");

        // check f+1
        let current_count = self.local_logs.get_local_messages_count_by_hash(&key_str);
        let reply_threshold = self.state.fault_tolerance_count + 1;
        if current_count as u64 == reply_threshold {
            self.state.current_state = 3;

            let result_str = std::str::from_utf8(&msg.result[..]).unwrap();
            println!("###############################################################");
            println!("Request consesnus successful, result is {}", result_str);
        }
    }
}
