use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use blsttc::{PublicKeyShare, Signature, SignatureShare};
use utils::crypto::blsttc::TBLSKey;
use crate::message::Request;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    Request(Request),
    NewView(NewView),
    Prepare(Prepare),
    Commit(Commit),
    PreCommit(PreCommit),
    Decide(Decide),
    PrepareVote(Vote),
    CommitVote(Vote),
    PreCommitVote(Vote),
    End(String),
    TBLSKey(TBLSKey),
    ConsensusNodePKsInfo(Vec<u8>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConsensusMessage {
    pub msg_type: MessageType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConsensusNodePKsInfo {
    map: HashMap<Vec<u8>, ConsensusNodePKInfo>,
}



#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Block {
    pub cmd: String,
    pub parent_hash: String,
}

// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct Request {
//     pub cmd: String,
// }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewView {
    pub view_num: u64,
    // pub block: Block,
    pub justify: Option<QC>,
    pub from_peer_id: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prepare {
    pub view_num: u64,
    pub block: Block,
    pub justify: Option<QC>,
    pub from_peer_id: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreCommit {
    pub view_num: u64,
    // pub block: Block,
    pub justify: Option<QC>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Commit {
    pub view_num: u64,
    // pub block: Block,
    pub justify: Option<QC>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Decide {
    pub view_num: u64,
    // pub block: Block,
    pub justify: Option<QC>,
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
    pub partial_signature: Option<SignatureShare>,
    pub from_peer_id: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct QC {
    pub msg_type: u8,
    pub view_num: u64,
    pub block: Block,
    pub signature: Option<Signature>,
}

impl QC {
    pub fn new(msg_type: u8, view_num: u64, block: &Block) -> Self {
        Self {
            msg_type,
            view_num,
            block: block.clone(),
            signature: None,
        }
    }

    pub fn set_signature(&mut self, sig: &Signature) {
        self.signature = Some(sig.clone());
    }
}
