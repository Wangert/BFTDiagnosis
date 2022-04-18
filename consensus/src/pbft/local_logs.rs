use std::collections::HashMap;

use libp2p::futures::future::Map;

use super::{
    common,
    message::{Commit, MessageType, PrePrepare, Prepare, Request, Reply},
};

// node's local logs in consensus process
pub struct LocalLogs {
    pub requests: HashMap<u64, Request>,
    pub messages: HashMap<String, Vec<MessageType>>,
}

impl LocalLogs {
    pub fn new() -> LocalLogs {
        let requests: HashMap<u64, Request> = HashMap::new();
        let messages: HashMap<String, Vec<MessageType>> = HashMap::new();
        LocalLogs { requests, messages }
    }

    pub fn record_message_handler(&mut self, msg: MessageType) {
        match msg {
            MessageType::PrePrepare(preprepare) => {
                self.record_preprepare(&preprepare);
            }
            MessageType::Prepare(prepare) => {
                self.record_prepare(&prepare);
            }
            MessageType::Commit(commit) => {
                self.record_commit(&commit);
            }
            MessageType::Reply(reply) => {
                self.record_reply(&reply);
            }
            _ => {}
        }
    }

    pub fn record_request(&mut self, sequence_number: u64, request: &Request) {
        if !self.requests.contains_key(&sequence_number) {
            self.requests.insert(sequence_number, request.clone());
        }
    }

    pub fn record_preprepare(&mut self, msg: &PrePrepare) {
        let key_hash = common::get_message_key(MessageType::PrePrepare(msg.clone()));

        //println!("[Preprepare hash key]：{:?}", &key_hash);

        if let Some(msg_vec) = self.messages.get_mut(&key_hash) {
            msg_vec.push(MessageType::PrePrepare(msg.clone()));
        } else {
            let msg_vec = vec![MessageType::PrePrepare(msg.clone())];
            self.messages.insert(key_hash, msg_vec);
        }
    }

    pub fn record_prepare(&mut self, msg: &Prepare) {
        let key_hash = common::get_message_key(MessageType::Prepare(msg.clone()));

        //println!("[Prepare hash key]：{:?}", &key_hash);

        if let Some(msg_vec) = self.messages.get_mut(&key_hash) {
            msg_vec.push(MessageType::Prepare(msg.clone()));
        } else {
            let msg_vec = vec![MessageType::Prepare(msg.clone())];
            self.messages.insert(key_hash, msg_vec);
        }
    }

    pub fn record_commit(&mut self, msg: &Commit) {
        let key_hash = common::get_message_key(MessageType::Commit(msg.clone()));

        //println!("[Preprepare hash key]：{:?}", &key_hash);

        if let Some(msg_vec) = self.messages.get_mut(&key_hash) {
            msg_vec.push(MessageType::Commit(msg.clone()));
        } else {
            let msg_vec = vec![MessageType::Commit(msg.clone())];
            self.messages.insert(key_hash, msg_vec);
        }
    }

    pub fn record_reply(&mut self, msg: &Reply) {
        let key_hash = common::get_message_key(MessageType::Reply(msg.clone()));

        if let Some(msg_vec) = self.messages.get_mut(&key_hash) {
            msg_vec.push(MessageType::Reply(msg.clone()))
        } else {
            let msg_vec = vec![MessageType::Reply(msg.clone())];
            self.messages.insert(key_hash, msg_vec);
        }
    }

    pub fn get_local_request_by_sequence_number(&self, sequence_number: u64) -> Option<&Request> {
        self.requests.get(&sequence_number)
    }

    pub fn get_local_messages_by_hash(&self, hash: &str) -> Box<Vec<MessageType>> {
        if let Some(msg_vec) = self.messages.get(&hash.to_string()) {
            Box::new(msg_vec.clone())
        } else {
            Box::new(vec![])
        }
    }

    pub fn get_local_messages_count_by_hash(&self, hash: &str) -> usize {
        if let Some(msg_vec) = self.messages.get(&hash.to_string()) {
            msg_vec.len()
        } else {
            0
        }
    }
}
