use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use crate::honeybadger::{reliable_broadcast,binary_agreement};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    /// The proposer whose contribution this message is about.
    pub proposer_id: String,
    /// The wrapped broadcast or agreement message.
    pub content: MessageContent,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessageContent {
    Request(Request),
    /// A wrapped message for the broadcast instance, to deliver the proposed value.
    Broadcast(String,reliable_broadcast::message::Message),
    /// A wrapped message for the agreement instance, to decide on whether to accept the value.
    Agreement(String,binary_agreement::Message),
    Reply(String,bool),
}


impl MessageContent{
    pub fn with(self,proposer_id: &str) -> Message{
        Message {  content: self, proposer_id: proposer_id.to_string() }
    }
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Request {
    pub data: Vec<u8>,
    pub client_id: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Reply {
    pub data: String,
    // pub client_id: Vec<bool>,
    pub output: HashMap<String,bool>
}