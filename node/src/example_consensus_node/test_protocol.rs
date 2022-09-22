use chrono::Local;

use utils::coder;

use crate::{
    basic_consensus_node::ConsensusNode,
    behaviour::{ConsensusNodeBehaviour, ProtocolLogsReadBehaviour, NodeStateUpdateBehaviour, PhaseState}, message::{Request, ConsensusData, ConsensusDataMessage},
};

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
        let cmd = format!("{}{}", "wangjitao", Local::now().timestamp_subsec_nanos());
        let request = Request {
            cmd,
            flag: false,
        };
        PhaseState::Over(request)
    }

    fn receive_consensus_requests(&mut self, _requests: Vec<crate::message::Request>) {
        todo!()
    }

    fn push_consensus_data_to_analysis_node(&mut self, _: &ConsensusData) {
        todo!()
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
