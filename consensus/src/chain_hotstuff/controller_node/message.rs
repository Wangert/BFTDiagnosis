use std::collections::HashMap;

use libp2p::PeerId;
use serde::{Serialize, Deserialize};
use threshold_crypto::{PublicKeyShare, SignatureShare, Signature};
use utils::crypto::threshold_signature::TBLSKey;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    Request(Request),
    Generic(Generic),
    Vote(Vote),
    End(String),
    TBLSKey(TBLSKey),
    ConsensusNodePKsInfo(HashMap<Vec<u8>, ConsensusNodePKInfo>),
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
    pub justify: Box<Option<QC>>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Generic {

}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Vote {
    pub block: Block,
    pub justify: QC,
    pub partial_signature: SignatureShare,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QC {
    pub view_num: u64,
    pub block: Block,
    pub signature: Signature,
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