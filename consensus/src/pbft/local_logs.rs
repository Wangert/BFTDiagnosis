use std::collections::HashMap;

use super::{
    common,
    message::{Commit, MessageType, PrePrepare, Prepare, Request},
};

// node's local logs in consensus process
pub struct LocalLogs {
    pub messages: HashMap<String, Vec<MessageType>>,
}

impl LocalLogs {
    pub fn new() -> LocalLogs {
        let messages: HashMap<String, Vec<MessageType>> = HashMap::new();
        LocalLogs { messages }
    }

    pub fn record_message_handler(&mut self, msg: MessageType) {
        match msg {
            MessageType::Request(request) => {
                self.record_request(&request);
            }
            MessageType::PrePrepare(preprepare) => {
                self.record_preprepare(&preprepare);
            }
            MessageType::Prepare(prepare) => {
                self.record_prepare(&prepare);
            }
            MessageType::Commit(commit) => {
                self.record_commit(&commit);
            }
            MessageType::Reply(reply) => {}
        }
    }

    pub fn record_request(&mut self, request: &Request) {
        let m_hash = common::get_message_key(MessageType::Request(request.clone()));

        if let Some(msg_vec) = self.messages.get_mut(&m_hash) {
            msg_vec.push(MessageType::Request(request.clone()));
        } else {
            let msg_vec = vec![MessageType::Request(request.clone())];
            self.messages.insert(m_hash, msg_vec);
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

    pub fn get_local_messages_by_hash(&self, hash: &str) -> Box<Vec<MessageType>> {
        if let Some(msg_vec) = self.messages.get(&hash.to_string()) {
            Box::new(msg_vec.clone())
        } else {
            Box::new(vec![])
        }
    }
}
