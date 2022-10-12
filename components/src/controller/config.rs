use std::collections::HashMap;

use serde::Deserialize;
use lazy_static::lazy_static;

use crate::message::MaliciousBehaviour;

// lazy_static! {
//    pub static ref BEHAVIOUR_MAP: HashMap<String, MaliciousBehaviour> = {
//         let mut map: HashMap<String, MaliciousBehaviour> = HashMap::new();

//         map.insert("LeaderFeignDeath".to_string(), MaliciousBehaviour::LeaderFeignDeath);
//         map.insert("LeaderSendAmbiguousMessage".to_string(), MaliciousBehaviour::LeaderSendAmbiguousMessage);
//         map.insert("LeaderDelaySendMessage".to_string(), MaliciousBehaviour::LeaderDelaySendMessage);
//         map.insert("LeaderSendDuplicateMessage".to_string(), MaliciousBehaviour::LeaderSendDuplicateMessage);
//         map.insert("ReplicaNodeConspireForgeMessages".to_string(), MaliciousBehaviour::ReplicaNodeConspireForgeMessages);
        
//         map
        
//     };
// }

#[derive(Debug, Deserialize, Clone)]
pub struct BFTDiagnosisConfig {
    pub throughput: Option<Throughput>,
    pub latency: Option<Latency>,
    pub scalability: Option<Scalability>,
    pub crash: Option<Crash>,
    pub malicious: Option<Malicious>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Throughput {
    pub enable: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Latency {
    pub enable: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Scalability {
    pub enable: Option<bool>,
    pub max: Option<u16>,
    pub internal: Option<u16>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Crash {
    pub enable: Option<bool>,
    pub max: Option<u16>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Malicious {
    pub enable: Option<bool>,
    pub number_of_phase: Option<u8>,
    pub behaviours: Option<Vec<String>>,
}
