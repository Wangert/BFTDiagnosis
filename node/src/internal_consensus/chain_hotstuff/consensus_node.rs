use chrono::Local;
use libp2p::PeerId;
use network::peer;


use crate::internal_consensus::chain_hotstuff::message::{Block, ConsensusNodePKInfo, Generic, ConsensusMessage, MessageType, Request, Vote, QC};
use crate::{
    basic_consensus_node::ConsensusNode,
    behaviour::{
        ConsensusEnd, ProtocolBehaviour, NodeStateUpdateBehaviour, ProtocolLogsReadBehaviour,
    },
    message::{ConsensusData, ConsensusDataMessage},
};
use libp2p::gossipsub::IdentTopic;
use utils::coder;



// impl <TLog, TState> ConsensusNodeBehaviour for ConsensusNode<TLog, TState>
// where 
//     TLog: Default + ProtocolLogsReadBehaviour,
//     TState: Default + NodeStateUpdateBehaviour,
// {
//     fn extra_initial_start(&mut self) {}

//     fn consensus_protocol_message_handler(&mut self, _msg: &[u8], peer_id:Option<PeerId>) -> ConsensusEnd {
//         ConsensusEnd::No
//     }

//     fn view_timeout_handler(&mut self) {}

//     fn receive_consensus_requests(&mut self, requests: Vec<crate::example_consensus_node::message::Request>) {
//         todo!()
//     }

//     fn analysis_node_poll_request(&mut self) {
//         todo!()
//     }

//     fn push_consensus_data_to_analysis_node(&mut self, _: &ConsensusData) {
//         todo!()
//     }
// }

// impl<TLog, TState> ConsensusNodeBehaviour for ConsensusNode<TLog, TState>
// where
//     TLog: Default + ProtocolLogsReadBehaviour,
//     TState: Default + NodeStateUpdateBehaviour,
// {
//     fn analysis_node_poll_request(&mut self) {
//         todo!()
//     }

//     fn consensus_protocol_message_handler(
//         &mut self,
//         _msg: &[u8],
//         peer_id: Option<PeerId>,
//     ) -> ConsensusEnd {
//         if let Some(peer_id) = peer_id {
//             println!("Get message from : {:?}", peer_id);
//         }

//         let data: ConsensusDataMessage = coder::deserialize_for_bytes(_msg);
        

//         ConsensusEnd::No
//     }

//     fn receive_consensus_requests(&mut self, _requests: Vec<Request>) {
//         todo!()
//     }

//     fn push_consensus_data_to_analysis_node(&mut self, consensus_data: &ConsensusData) {
//         let consensus_data_message = ConsensusDataMessage {
//             data: consensus_data.clone(),
//         };

//         let serialized_consensus_data_message =
//             coder::serialize_into_bytes(&consensus_data_message);

//         let analysis_node_id = self.analyzer_id();
//         self.peer_mut()
//             .network_swarm_mut()
//             .behaviour_mut()
//             .unicast
//             .send_message(&analysis_node_id, serialized_consensus_data_message);
//     }
// }
