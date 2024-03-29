use serde::{Deserialize, Serialize};
use utils::crypto::eddsa::EdDSAPublicKey;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    Request(Request),
    PrePrepare(PrePrepare),
    Prepare(Prepare),
    Commit(Commit),
    Reply(Reply),
    CheckPoint(CheckPoint),
    ViewChange(ViewChange),
    NewView(NewView),
    PublicKey(PublicKey),
    DistributePK,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    // pub id: Vec<u8>,
    pub msg_type: MessageType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Request {
    pub operation: String,
    pub timestamp: String,
    pub client_id: Vec<u8>,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PrePrepare {
    pub view: u64,
    pub number: u64,
    pub m_hash: String,
    pub m: Vec<u8>,
    pub signature: Vec<u8>,
    pub from_peer_id: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prepare {
    pub view: u64,
    pub number: u64,
    pub m_hash: String,
    pub from_peer_id: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Commit {
    pub view: u64,
    pub number: u64,
    pub m_hash: String,
    pub from_peer_id: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Reply {
    pub client_id: Vec<u8>,
    pub timestamp: String,
    pub view: u64,
    pub number: u64,
    pub from_peer_id: Vec<u8>,
    pub signature: Vec<u8>,
    pub result: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MetaReply {
    pub client_id: Vec<u8>,
    pub timestamp: String,
    pub view: u64,
    pub number: u64,
    pub result: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CheckPoint {
    pub current_max_number: u64,
    pub checkpoint_state_digest: String,
    pub from_peer_id: Vec<u8>,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ViewChange {
    pub new_view: u64,
    pub latest_stable_checkpoint: u64,
    pub latest_stable_checkpoint_messages: Vec<CheckPoint>,
    pub proof_messages: ProofMessages,
    pub from_peer_id: Vec<u8>,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewView {
    pub view: u64,
    pub viewchanges: Vec<ViewChange>,
    pub preprepares: Vec<PrePrepare>,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProofMessages {
    pub preprepares: Vec<MessageType>,
    pub prepares: Vec<MessageType>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublicKey {
    pub pk: EdDSAPublicKey,
    pub from_peer_id: Vec<u8>,
}

#[cfg(test)]
mod message_tests {
    use utils::coder::{self, get_hash_str};

    use crate::pbft::message::{Message, MessageType, Request};
    use chrono::prelude::*;

    use super::PrePrepare;

    #[test]
    fn message_serialize_works() {
        let value = vec!['y' as u8];
        let m_hash = get_hash_str(&value);
        let preprepare = PrePrepare {
            view: 1,
            number: 1,
            m_hash,
            m: value,
            signature: vec![1],
            from_peer_id: vec![],
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
            client_id: "client_id".as_bytes().to_vec(),
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
