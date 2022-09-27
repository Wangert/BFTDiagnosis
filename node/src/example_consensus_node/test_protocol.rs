use std::collections::HashMap;

use chrono::Local;

use serde::{Serialize, Deserialize};
use threshold_crypto::Signature;
use utils::coder;

use crate::{
    basic_consensus_node::ConsensusNode,
    behaviour::{ConsensusNodeBehaviour, ProtocolLogsReadBehaviour, NodeStateUpdateBehaviour, PhaseState}, message::{Request, ConsensusData, ConsensusDataMessage},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prepare {
    pub view_num: u64,
    pub block: Block,
    pub justify: Option<QC>,
    pub from_peer_id: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Commit {
    pub view_num: u64,
    pub justify: Option<QC>,
    pub from_peer_id: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct QC {
    pub msg_type: u8,
    pub view_num: u64,
    pub block: Option<Block>,
    pub signature: Option<Signature>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Block {
    pub cmd: String,
    pub parent_hash: String,
}

pub struct TestProtocol {

}

impl Default for TestProtocol {
    fn default() -> Self {
        Self {  }
    }
}

impl ConsensusNodeBehaviour for TestProtocol {
    fn analysis_node_poll_request(&mut self) {
        todo!()
    }

    fn consensus_protocol_message_handler(&mut self, _msg: &[u8]) -> PhaseState {
        let timestamp = Local::now().timestamp_millis();
        let cmd = format!("{}{}", "wangjitao", timestamp);
        let request = Request {
            cmd,
            flag: false,
            timestamp,
        };
        PhaseState::Over(request)
    }

    fn receive_consensus_requests(&mut self, _requests: Vec<crate::message::Request>) {
        todo!()
    }

    fn push_consensus_data_to_analysis_node(&mut self, _: &ConsensusData) {
        todo!()
    }

    fn protocol_phases(&mut self) -> HashMap<u8, Vec<u8>> {
        let mut hash_map = HashMap::new();
        let block_1 = Block {
            cmd: "wangjitao".to_string(),
            parent_hash: "hhhhh".to_string(),
        };
        let block_2 = Block {
            cmd: "".to_string(),
            parent_hash: "first".to_string(),
        };
        let qc = QC {
            msg_type: 3,
            view_num: 86,
            block: Some(block_2),
            signature: None,
        };
        let prepare = Prepare {
            view_num: 86,
            block: block_1,
            justify: Some(qc.clone()),
            from_peer_id: vec![1],
        };
        let commit = Commit {
            view_num: 86,
            justify: Some(qc),
            from_peer_id: vec![2],
        };

        let prepare_json_bytes = coder::serialize_into_json_str(&prepare).as_bytes().to_vec();
        let commit_json_bytes = coder::serialize_into_json_str(&commit).as_bytes().to_vec();

        hash_map.insert(1, prepare_json_bytes);
        hash_map.insert(2, commit_json_bytes);

        hash_map
    }

    fn current_request(&self) -> Request {
        Request { cmd: "Test".to_string(), flag: true, timestamp: Local::now().timestamp_millis() }
    }

    // fn push_consensus_data_to_analysis_node(&mut self, consensus_data: &ConsensusData) {
    //     let consensus_data_message = ConsensusDataMessage {
    //         data: consensus_data.clone(),
    //     };

    //     let serialized_consensus_data_message = coder::serialize_into_bytes(&consensus_data_message);

    //     let analysis_node_id = self.analyzer_id();
    //     self.peer_mut().network_swarm_mut().behaviour_mut().unicast.send_message(&analysis_node_id, serialized_consensus_data_message);
    // }
}
