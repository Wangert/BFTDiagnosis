use std::collections::HashMap;

use libp2p::PeerId;
use serde::{Serialize, Deserialize};
use threshold_crypto::{PublicKeyShare, SignatureShare, Signature};
use utils::crypto::threshold_signature::TBLSKey;

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
    ConsensusNodePKsInfo(HashMap<Vec<u8>, ConsensusNodePKInfo>),
    //Reply(Reply),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub msg_type: MessageType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Request {
    pub cmd: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    pub cmd: String,
    pub parent_hash: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewView {

}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prepare {

}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreCommit {

}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Commit {

}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Decide {

}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Vote {
    pub block: Block,
    pub justify: QC,
    pub partial_signature: SignatureShare,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QC {
    pub msg_type: u64,
    pub view_num: u64,
    pub block: Block,
    pub signature: Signature,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Reply {

}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConsensusNodePKInfo {
    pub number: u64,
    pub public_key: PublicKeyShare,
}

#[derive(Debug, Clone)]
pub struct DistributeTBLSKey {
    pub number: u64,
    pub peer_id: PeerId,
    pub tbls_key: TBLSKey,
}