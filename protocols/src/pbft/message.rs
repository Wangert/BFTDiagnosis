use serde::{Deserialize, Serialize};
use utils::crypto::eddsa::EdDSAPublicKey;
use components::message::Request;
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
pub struct ConsensusMessage {
    // pub id: Vec<u8>,
    pub msg_type: MessageType,
}

// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct Request {
//     pub cmd: String,
//     // pub timestamp: String,
//     // pub client_id: Vec<u8>,
//     // pub signature: String,
// }

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
    // pub client_id: Vec<u8>,
    pub cmd: String,
    // pub timestamp: String,
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
    // pub latest_stable_checkpoint: u64,
    // pub latest_stable_checkpoint_messages: Vec<CheckPoint>,
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


