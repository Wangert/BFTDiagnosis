use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use blsttc::{PublicKeyShare, Signature, SignatureShare};
use utils::crypto::threshold_blsttc::TBLSKey;
use crate::message::Request;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    Init(),
    Request(Request),
    Generic(Generic),
    Vote(Vote),
    End(String),
    TBLSKey(TBLSKey),
    ConsensusNodePKsInfo(HashMap<Vec<u8>, ConsensusNodePKInfo>),
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Done(u64),
    Do(u64),
    NotIsLeader(u64),
    Init,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConsensusMessage {
    pub msg_type: MessageType,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Block {
    pub cmd: String,
    pub parent_hash: String,
    pub justify: Box<Option<QC>>,
}

// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct Request {
//     pub cmd: String,
// }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Generic {
    pub view_num: u64,
    pub block: Option<Block>,
    pub justify: Option<QC>,
    pub from_peer_id: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConsensusNodePKInfo {
    pub number: u64,
    pub public_key: PublicKeyShare,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Vote {
    pub view_num: u64,
    pub block: Block,
    // pub justify: QC,
    pub partial_signature: SignatureShare,
    pub from_peer_id: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct QC {
    pub view_num: u64,
    pub block: Block,
    pub signature: Option<Signature>,
}

impl QC {
    pub fn new(view_num: u64, block: &Block) -> Self {
        Self {
            view_num,
            block: block.clone(),
            signature: None,
        }
    }

    pub fn set_signature(&mut self, sig: &Signature) {
        self.signature = Some(sig.clone());
    }
}
