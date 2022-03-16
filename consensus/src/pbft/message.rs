
use serde::{Serialize, Deserialize};
use threshold_crypto::Signature;


#[derive(Debug, Serialize, Deserialize)]
pub enum MessageType {
    Request(Request),
    PrePrepare(PrePrepare),
    Prepare(Prepare),
    Commit(Commit),
    Reply(Reply),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    // pub id: Vec<u8>,
    pub msg_type: MessageType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub operation: String,
    pub timestamp: String,
    pub client_addr: String,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrePrepare {
    pub view: u64,
    pub number: u64,
    pub m_hash: Vec<u8>,
    pub m: Vec<u8>,
    pub signature: String,
    pub peer_id: String,
    //pub from_peer_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Prepare {
    pub view: u64,
    pub number: u64,
    pub m_hash: Vec<u8>,
    pub from_peer_id: String,
    pub signature: Signature,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Commit {
    pub view: u64,
    pub number: u64,
    pub m_hash: Vec<u8>,
    pub from_peer_id: String,
    pub signature: Signature,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reply {
    pub client_addr: String,
    pub timestamp: i64,
    pub view: u64,
    pub number: u64,
    pub from_peer_id: String,
    pub signature: Signature,
    pub result: Vec<u8>,
}


#[cfg(test)]
mod message_tests {
    use utils::coder;

    use crate::pbft::message::{Message, MessageType, Request};
    use chrono::prelude::*;

    use super::PrePrepare;

    #[test]
    fn message_serialize_works() {
        let value = vec!['y' as u8];
        let mut out = vec![];
        coder::get_hash(&value, &mut out);
        let preprepare = PrePrepare {
            view: 1,
            number: 1,
            m_hash: out,
            m: value,
            peer_id: String::from(""),
            signature: String::from("signature"),
        };

        let msg = Message {
            msg_type: MessageType::PrePrepare(preprepare),
        };

        let serialized_msg = coder::serialize_into_bytes(&msg);
        println!("Serialized msg: {:?}", &serialized_msg);
        let str_msg = std::str::from_utf8(&serialized_msg).unwrap();
        println!("Str msg: {:?}", &str_msg);
        let deserialized_msg: Message = coder::deserialize_for_bytes(str_msg.as_bytes());
        println!("Deserialized msg: {:?}", &deserialized_msg);


        let request = Request {
            operation: String::from("operation"),
            client_addr: String::from("client_addr"),
            timestamp: Local::now().timestamp().to_string(),
            signature: String::from("signature"),
        };
    
        let msg_1 = Message {
            msg_type: MessageType::Request(request),
        };
    
        let serialized_msg_1 = coder::serialize_into_bytes(&msg_1);
        let broadcast_msg_1 = std::str::from_utf8(&serialized_msg_1).unwrap();

        let deserialized_msg_1: Message = coder::deserialize_for_bytes(broadcast_msg_1.as_bytes());

        println!("Deserialzed_msg: {:?}", &deserialized_msg_1);
    }
}