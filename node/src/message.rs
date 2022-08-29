use std::collections::HashMap;

use libp2p::PeerId;
use serde::{Serialize, Deserialize};
use threshold_crypto::{PublicKeyShare, SignatureShare, Signature};
use utils::crypto::threshold_signature::TBLSKey;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    // Make a consensus request
    MakeAConsensusRequest(Request),
    // Make a set of consensus requests
    MakeConsensusRequests(Vec<Request>),
    // Assign TBLS keypair
    AssignTBLSKeypair(TBLSKey),
    // Distribute consensus node's public keys' information
    DistributeConsensusNodePKsInfo(HashMap<Vec<u8>, ConsensusNodePKInfo>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConsensusData {
    ConsensusStartData(ConsensusStartData),
    ConsensusEndData(ConsensusEndData),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandMessage {
    pub command: Command,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConsensusDataMessage {
    pub data: ConsensusData
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TestItem {
    Throughout,
    Latency,
    Scalability,
    Crash,
    Malicious(MaliciousAction),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MaliciousAction {
    Action1,
    Action2,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Request {
    pub cmd: String,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConsensusStartData {
    pub request: Request,
    pub start_time: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConsensusEndData {
    pub request: Request,
    pub completed_time: u64,
}