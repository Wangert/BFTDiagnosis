use chrono::Local;
use libp2p::PeerId;
use utils::coder;

use crate::{
    basic_consensus_node::ConsensusNode,
    behaviour::{ConsensusNodeBehaviour, ProtocolHandler, ProtocolLogsReadBehaviour, NodeStateUpdateBehaviour, ConsensusEnd}, message::{Request, ConsensusData, ConsensusDataMessage},
};

impl<TLog, TState> ConsensusNodeBehaviour for ConsensusNode<TLog, TState>
where
    TLog: Default + ProtocolLogsReadBehaviour,
    TState: Default + NodeStateUpdateBehaviour,
{
    fn analysis_node_poll_request(&mut self) {
        todo!()
    }

    fn consensus_protocol_message_handler(&mut self, msg: &[u8]) -> ConsensusEnd {
        let cmd = format!("{}{}", "wangjitao", Local::now().timestamp_subsec_nanos());
        let request = Request {
            cmd,
        };
        ConsensusEnd::Yes(request)
    }

    fn receive_consensus_requests(&mut self, requests: Vec<crate::message::Request>) {
        todo!()
    }

    fn push_consensus_data_to_analysis_node(&mut self, consensus_data: &ConsensusData) {
        let consensus_data_message = ConsensusDataMessage {
            data: consensus_data.clone(),
        };

        let serialized_consensus_data_message = coder::serialize_into_bytes(&consensus_data_message);

        let analysis_node_id = self.analyzer_id();
        self.peer_mut().network_swarm_mut().behaviour_mut().unicast.send_message(&analysis_node_id, serialized_consensus_data_message);
    }
}
