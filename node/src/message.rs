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
pub struct CommandMessage {
    pub command: Command,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Component {
    // controller component peer id(vec<u8 type>)
    Controller(Vec<u8>),
    // analyzer component peer id(vec<u8> type)
    Analyzer(Vec<u8>),
    // consensus node peer id(vec<u8> type)
    ConsensusNode(Vec<u8>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InteractiveMessage {
    ComponentInfo(Component),
    TestItem(TestItem),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub interactive_message: InteractiveMessage,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum TestItem {
    Throughput,
    Latency,
    ThroughputAndLatency,
    Scalability,
    Crash,
    Malicious(MaliciousAction),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
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
pub enum ConsensusData {
    ConsensusStartData(ConsensusStartData),
    ConsensusEndData(ConsensusEndData),
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConsensusDataMessage {
    pub data: ConsensusData
}