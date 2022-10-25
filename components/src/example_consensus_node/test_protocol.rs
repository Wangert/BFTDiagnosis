use std::collections::{HashMap, VecDeque};

use chrono::Local;

use serde::{Deserialize, Serialize};
use threshold_crypto::Signature;
use utils::coder;

use crate::{
    basic_consensus_node::ConsensusNode,
    behaviour::{
        ProtocolBehaviour, NodeStateUpdateBehaviour, PhaseState, ProtocolLogsReadBehaviour,
        SendType,
    },
    message::{ConsensusData, ConsensusDataMessage, Request}, common::get_request_hash,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    Request(Request),
    Prepare(Prepare),
    Commit(Commit),
    PreCommit(PreCommit),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProtocolMessage {
    pub msg_type: MessageType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prepare {
    pub view_num: u64,
    pub block: Block,
    pub justify: Option<QC>,
    pub from_peer_id: Vec<u8>,
    pub request: Request,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Commit {
    pub view_num: u64,
    pub justify: Option<QC>,
    pub from_peer_id: Vec<u8>,
    pub r_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreCommit {
    pub view_num: u64,
    // pub block: Block,
    pub justify: Option<QC>,
    pub r_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct QC {
    pub msg_type: u8,
    pub view_num: u64,
    pub block: Option<Block>,
    // pub signature: Option<Signature>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Block {
    pub cmd: String,
    pub parent_hash: String,
}

pub struct TestProtocol {
    pub current_request: Request,
    pub is_leader: bool,
}

impl Default for TestProtocol {
    fn default() -> Self {
        Self {
            current_request: Request {
                cmd: "None".to_string(),
                // timestamp: Local::now().timestamp_nanos() as u64,
            },
            is_leader: false,
        }
    }
}

impl TestProtocol {
    pub fn set_current_request(&mut self, request: &Request) {
        self.current_request = request.clone();
    }

    pub fn handle_request(&mut self, msg: &Request) -> PhaseState {
        println!("Handle Request");
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
            // signature: None,
        };
        
        let prepare = Prepare {
            view_num: 86,
            block: block_1,
            justify: Some(qc.clone()),
            from_peer_id: vec![1],
            request: msg.clone(),
        };

        let protocol_message = ProtocolMessage {
            msg_type: MessageType::Prepare(prepare),
        };
        let serialized_message = coder::serialize_into_json_bytes(&protocol_message);

        let send_type = SendType::Broadcast(serialized_message);

        let mut send_queue = VecDeque::new();
        send_queue.push_back(send_type);
        let phase_state = PhaseState::ContinueExecute(send_queue);

        phase_state
    }

    pub fn handle_prepare(&mut self, msg: &Prepare) -> PhaseState {
        println!("Handle Prepare");
        self.set_current_request(&msg.request);
        let r_hash = get_request_hash(&self.current_request);
        let precommit = PreCommit {
            view_num: 86,
            justify: None,
            r_hash,
        };

        let protocol_message = ProtocolMessage {
            msg_type: MessageType::PreCommit(precommit),
        };
        let serialized_message = coder::serialize_into_json_bytes(&protocol_message);

        let send_type = SendType::Broadcast(serialized_message);

        let mut send_queue = VecDeque::new();
        send_queue.push_back(send_type);
        let phase_state = PhaseState::ContinueExecute(send_queue);

        phase_state
    }

    pub fn handle_precommit(&mut self, msg: &PreCommit) -> PhaseState {
        println!("Handle PreCommit");
        let commit = Commit {
            view_num: 86,
            justify: None,
            from_peer_id: vec![],
            r_hash: msg.r_hash.clone(),
        };

        let protocol_message = ProtocolMessage {
            msg_type: MessageType::Commit(commit),
        };
        let serialized_message = coder::serialize_into_json_bytes(&protocol_message);

        let send_type = SendType::Broadcast(serialized_message);

        let mut send_queue = VecDeque::new();
        send_queue.push_back(send_type);
        let phase_state = PhaseState::ContinueExecute(send_queue);

        phase_state
    }

    pub fn handle_commit(&mut self, _msg: &Commit) -> PhaseState {
        println!("Handle Commit");
        let phase_state = PhaseState::Over(self.current_request.clone());
        phase_state
    }
}

impl ProtocolBehaviour for TestProtocol {
    fn analysis_node_poll_request(&mut self) {
        todo!()
    }

    fn consensus_protocol_message_handler(&mut self, msg: &[u8]) -> PhaseState {
        let message: ProtocolMessage = coder::deserialize_for_json_bytes(msg);

        match message.msg_type {
            MessageType::Request(msg) => self.handle_request(&msg),
            MessageType::Prepare(msg) => self.handle_prepare(&msg),
            MessageType::PreCommit(msg) => self.handle_precommit(&msg),
            MessageType::Commit(msg) => self.handle_commit(&msg),
        }
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
            // signature: None,
        };
        let prepare = Prepare {
            view_num: 86,
            block: block_1,
            justify: Some(qc.clone()),
            from_peer_id: vec![1],
            request: todo!(),
        };
        let commit = Commit {
            view_num: 86,
            justify: Some(qc),
            from_peer_id: vec![2],
            r_hash: todo!(),
        };

        let prepare_json_bytes = coder::serialize_into_json_str(&prepare).as_bytes().to_vec();
        let commit_json_bytes = coder::serialize_into_json_str(&commit).as_bytes().to_vec();

        hash_map.insert(1, prepare_json_bytes);
        hash_map.insert(2, commit_json_bytes);

        hash_map
    }

    fn current_request(&self) -> Request {
        Request {
            cmd: "Test".to_string(),
            // timestamp: Local::now().timestamp_nanos() as u64,
        }
    }

    fn is_leader(&self) -> bool {
        self.is_leader
    }

    fn set_leader(&mut self, is_leader: bool) {
        self.is_leader = is_leader;
    }

    fn generate_serialized_request_message(&self, request: &Request) -> Vec<u8> {
        let protocol_message = ProtocolMessage {
            msg_type: MessageType::Request(request.clone()),
        };

        coder::serialize_into_json_bytes(&protocol_message)
    }

    fn set_current_request(&mut self, request: &Request) {
        self.current_request = request.clone();
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
