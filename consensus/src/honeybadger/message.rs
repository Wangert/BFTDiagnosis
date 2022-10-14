use std::collections::HashMap;

use libp2p::PeerId;
use serde::{Serialize, Deserialize};
use threshold_crypto::{PublicKeyShare, SignatureShare, Signature};
use utils::crypto::threshold_signature::TBLSKey;



#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Message {
    TBLSKey(TBLSKey),
    Request(Request),
    //ConsensusNodePKsInfo(HashMap<Vec<u8>, ConsensusNodePKInfo>),
    //Reply(Reply),
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConsensusNodePKInfo {
    pub number: u64,
    pub public_key: PublicKeyShare,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Request {
    pub data: Vec<u8>,
    pub client_id: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct DistributeTBLSKey {
    pub number: u64,
    pub peer_id: PeerId,
    pub tbls_key: TBLSKey,
}