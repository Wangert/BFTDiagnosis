use std::collections::HashMap;

use libp2p::PeerId;
use serde::{Serialize, Deserialize};
use threshold_crypto::{PublicKeyShare, SignatureShare, Signature};
use utils::crypto::threshold_signature::TBLSKey;

use crate::basic_consensus_node::ConsensusNodeMode;

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

// An enumeration of component types that contains the ID information for component nodes
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Component {
    // controller component peer id(vec<u8 type>)
    Controller(Vec<u8>),
    // analyzer component peer id(vec<u8> type)
    Analyzer(Vec<u8>),
    // consensus node peer id(vec<u8> type)
    ConsensusNode(Vec<u8>),
}

// The component interaction information type of the BFTDiagnosis framework
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InteractiveMessage {
    // component information, including the component type
    ComponentInfo(Component),
    // protocol test items, including test content
    TestItem(TestItem),
    // interactive information to start the test
    StartTest(u64),
    // test item has completed the interaction of the test
    CompletedTest(TestItem),

    CrashNode(u16, Vec<u8>),

    SubscribeConsensusTopic(u64),
    SubscribeConsensusTopicSuccess,

    ConsensusNodeMode(ConsensusNodeMode),
    ConsensusNodeModeSuccess(ConsensusNodeMode),

    ProtocolStart(u64),
    Reset(u64),
    ResetSuccess(ConsensusNodeMode),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub interactive_message: InteractiveMessage,
    pub source: Vec<u8>,
}

// Protocol test item type
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum TestItem {
    // testing the throughput of the protocol
    Throughput,
    // testing the latency of the protocol
    Latency,
    // testing the throughput and latency of the protocol
    ThroughputAndLatency,
    // testing the scalability of the protocol 
    // (number of nodes, maximum number of nodes, increment interval)
    Scalability(u16, u16, u16),
    // testing protocol security in the case of node crash, including the number of crash nodes
    // (number of crash nodes, maximum number of crash nodes)
    Crash(u16, u16),
    // testing protocol security in the case of malicious nodes
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
    pub start_time: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConsensusEndData {
    pub request: Request,
    pub completed_time: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConsensusDataMessage {
    pub data: ConsensusData
}