use std::{collections::HashMap, fmt::{Formatter, Result, Display}};

use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use blsttc::PublicKeyShare;
use utils::crypto::threshold_blsttc::TBLSKey;

use crate::protocol_actuator::{ConsensusNodeMode, ConfigureState};

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

    CrashNode(u16, Option<Vec<u8>>),
    DishonestNode(Vec<u8>),
    JoinConsensus(u64),
    JoinConsensusSuccess,

    MaliciousTestPreparation,
    ConsensusPhase(u8, Vec<u8>),

    ConfigureConsensusNode(ConfigureState),
    ConfigureConsensusNodeSuccess(ConfigureState),

    ConsensusNodeMode(ConsensusNodeMode),
    ConsensusNodeModeSuccess(ConsensusNodeMode),
    
    ConfigureParams(Param),
    
    ProtocolStart(u64),
    Reset(u64),
    ResetSuccess(ConsensusNodeMode),

    // Make a consensus request
    MakeAConsensusRequest(Request),
    // Make a set of consensus requests
    MakeConsensusRequests(Vec<Request>),
    InitialLeader(Vec<u8>)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub interactive_message: InteractiveMessage,
    pub source: Vec<u8>,
}

//Parms
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Param {
    ConsensusParam(ConsensusParam),
    DeployParam(DeployParam),
    ConsensusAndDeployParam(ConsensusParam,DeployParam),
    NoParam(),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct ConsensusParam {
    batch_size: u16,
}

impl ConsensusParam {
    pub fn new(size: u16) -> ConsensusParam {
        ConsensusParam { batch_size: size }
    }

    pub fn batch_size(&self) -> u16 {
        self.batch_size
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct DeployParam {
    delay: u16
}

impl DeployParam {
    pub fn new(delay: u16) -> DeployParam {
        DeployParam { delay: delay }
    }

    pub fn delay(&self) -> u16 {
        self.delay
    }
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
    Malicious(MaliciousBehaviour),
}

impl Display for TestItem {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            TestItem::Throughput => write!(f, "Throughput"),
            TestItem::Latency => write!(f, "Latency"),
            TestItem::ThroughputAndLatency => write!(f, "ThroughputAndLatency"),
            TestItem::Scalability(n_1, n_2, n_3) => write!(f, "Scalability({}, {}, {})", n_1, n_2, n_3),
            TestItem::Crash(n_1, n_2) => write!(f, "Crash({}, {})", n_1, n_2),
            TestItem::Malicious(m) => write!(f, "Malicious({})", m.to_string()),
        }
    }
}

impl From<TestItem> for String {
    fn from(item: TestItem) -> Self {
        match item {
            TestItem::Throughput => "Throughput".into(),
            TestItem::Latency => "Latency".into(),
            TestItem::ThroughputAndLatency => "ThroughputAndLatency".into(),
            TestItem::Scalability(n_1, n_2, n_3) => format!("Scalability({}, {}, {})", n_1, n_2, n_3).into(),
            TestItem::Crash(n_1, n_2) => format!("Crash({}, {})", n_1, n_2).into(),
            TestItem::Malicious(m) => format!("Malicious({})", m.to_string()).into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum MaliciousBehaviour {
    LeaderFeignDeath(Round, u8),
    LeaderSendAmbiguousMessage(Round, u8, Vec<String>, u16),
    LeaderDelaySendMessage(Round, u8),
    LeaderSendDuplicateMessage(Round, u8),
    ReplicaFeignDeath(Round, u8, u16),
    ReplicaSendAmbiguousMessage(Round, u8, Vec<String>, u16, u16),
    ReplicaDelaySendMessage(Round, u8, u16),
    ReplicaSendDuplicateMessage(Round, u8, u16),
    ReplicaNodeConspireForgeMessages(Round, u8, Vec<String>, u16),
}

impl Display for MaliciousBehaviour {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            MaliciousBehaviour::LeaderFeignDeath(round, n) => write!(f, "LeaderFeignDeath({}, {})", round.to_string(), n),
            MaliciousBehaviour::LeaderSendAmbiguousMessage(round, n_1, field, n_2) => write!(f, "LeaderSendAmbiguousMessage({}, {}, {:?}, {})", round.to_string(), n_1, field, n_2),
            MaliciousBehaviour::LeaderDelaySendMessage(round, n_1) => write!(f, "LeaderDelaySendMessage({}, {})", round.to_string(), n_1),
            MaliciousBehaviour::LeaderSendDuplicateMessage(round, n_1) => write!(f, "LeaderSendDuplicateMessage({}, {})", round.to_string(), n_1),
            MaliciousBehaviour::ReplicaNodeConspireForgeMessages(round, n_1, field, n_2) => write!(f, "ReplicaNodeConspireForgeMessages({}, {}, {:?}, {})", round.to_string(), n_1, field, n_2),
            MaliciousBehaviour::ReplicaFeignDeath(round, n, num) => write!(f, "ReplicaFeignDeath({}, {}, {})", round.to_string(), n, num),
            MaliciousBehaviour::ReplicaSendAmbiguousMessage(round, n_1, field, n_2, num) => write!(f, "ReplicaSendAmbiguousMessage({}, {}, {:?}, {}, {})", round.to_string(), n_1, field, n_2, num),
            MaliciousBehaviour::ReplicaDelaySendMessage(round, n_1, num) => write!(f, "ReplicaDelaySendMessage({}, {}, {})", round.to_string(), n_1, num),
            MaliciousBehaviour::ReplicaSendDuplicateMessage(round, n_1, num) => write!(f, "ReplicaSendDuplicateMessage({}, {},{})", round.to_string(), n_1, num),
        }
    }
}

impl From<MaliciousBehaviour> for String {
    fn from(m: MaliciousBehaviour) -> Self {
        match m {
            MaliciousBehaviour::LeaderFeignDeath(round, n) => format!("LeaderFeignDeath({}, {})", round.to_string(), n).into(),
            MaliciousBehaviour::LeaderSendAmbiguousMessage(round, n_1, field, n_2) => format!("LeaderSendAmbiguousMessage({}, {}, {:?}, {})", round.to_string(), n_1, field, n_2).into(),
            MaliciousBehaviour::LeaderDelaySendMessage(round, n_1) => format!("LeaderDelaySendMessage({}, {})", round.to_string(), n_1).into(),
            MaliciousBehaviour::LeaderSendDuplicateMessage(round, n_1) => format!("LeaderSendDuplicateMessage({}, {})", round.to_string(), n_1).into(),
            MaliciousBehaviour::ReplicaNodeConspireForgeMessages(round, n_1, field, n_2) => format!("ReplicaNodeConspireForgeMessages({}, {}, {:?}, {})", round.to_string(), n_1, field, n_2).into(),
            MaliciousBehaviour::ReplicaFeignDeath(round, n, num) => format!("ReplicaFeignDeath({}, {}, {})", round.to_string(), n, num).into(),
            MaliciousBehaviour::ReplicaSendAmbiguousMessage(round, n_1, field, n_2, num) => format!("LeaderSendAmbiguousMessage({}, {}, {:?}, {}, {})", round.to_string(), n_1, field, n_2, num).into(),
            MaliciousBehaviour::ReplicaDelaySendMessage(round, n_1, num) => format!("ReplicaDelaySendMessage({}, {}, {})", round.to_string(), n_1, num).into(),
            MaliciousBehaviour::ReplicaSendDuplicateMessage(round, n_1, num) => format!("ReplicaSendDuplicateMessage({}, {}, {})", round.to_string(), n_1, num).into(),
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Round {
    FirstRound,
    OtherRound(u8),
}

impl Display for Round {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Round::FirstRound => write!(f, "FirstRound"),
            Round::OtherRound(n) => write!(f, "OtherRound({})", n),
        }
    }
}

impl From<Round> for String {
    fn from(round: Round) -> Self {
        match round {
            Round::FirstRound => "FirstRound".into(),
            Round::OtherRound(n) => format!("OtherRound({})", n).into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Request {
    pub cmd: String,
    // pub timestamp: u64,
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
    pub data: ConsensusData,
}


#[cfg(test)]
pub mod message_tests {
    use super::TestItem;

    #[test]
    fn test_item_works() {
        let item = TestItem::Scalability(1, 2, 3);
        let s = item.to_string();
        println!("{}", s);
    }
}