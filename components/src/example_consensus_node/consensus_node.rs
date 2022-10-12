use chrono::Local;
use libp2p::PeerId;
use network::peer;

use crate::example_consensus_node::message::{
    Commit, ConsensusMessage, MessageType, PrePrepare, Prepare, Reply, Request,
};
use crate::{
    basic_consensus_node::ConsensusNode,
    behaviour::{
        ConsensusEnd, ProtocolBehaviour, NodeStateUpdateBehaviour, ProtocolLogsReadBehaviour,
    },
    message::{ConsensusData, ConsensusDataMessage},
};
use libp2p::gossipsub::IdentTopic;
use utils::coder;

// impl<TLog, TState> ConsensusNodeBehaviour for ConsensusNode<TLog, TState,TP>
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

//         let data: ConsensusMessage = coder::deserialize_for_bytes(_msg);
//         match data.msg {
//             MessageType::Request(request) => {
//                 let cmd = request.cmd;
//                 println!("Consensus node received the Request message.");
//                 let preprepare = PrePrepare { cmd };
//                 let message = MessageType::PrePrepare(preprepare.clone());
//                 let data = ConsensusMessage { msg: message };
//                 let msg = coder::serialize_into_bytes(&data);
//                 let topic = IdentTopic::new("Consensus");
//                 if let Err(e) = self
//                     .peer_mut()
//                     .network_swarm_mut()
//                     .behaviour_mut()
//                     .gossipsub
//                     .publish(topic.clone(), msg)
//                 {
//                     eprintln!("Publish message error:{:?}", e);
//                 }
//                 println!("Broadcast PrePrepare message success!");

//                 //**** */
//                 let cmd = preprepare.cmd;
//                 println!("Consensus node received PrePreare message : {:?}", cmd);
//                 let prepare = Prepare { cmd };
//                 let message = MessageType::Prepare(prepare);
//                 let data = ConsensusMessage { msg: message };
//                 let msg = coder::serialize_into_bytes(&data);
//                 let topic = IdentTopic::new("Consensus");
//                 if let Err(e) = self
//                     .peer_mut()
//                     .network_swarm_mut()
//                     .behaviour_mut()
//                     .gossipsub
//                     .publish(topic.clone(), msg)
//                 {
//                     eprintln!("Publish message error:{:?}", e);
//                 }
//                 println!("Broadcast Prepare!");
//             }
//             MessageType::PrePrepare(preprepare) => {

//                 let request = Request {
//                     cmd: preprepare.clone().cmd,
//                 };
//                 let map = self.end_map();
                

//                 let cmd = preprepare.cmd;
//                 println!("Consensus node received PrePreare message : {:?}", cmd);
//                 let prepare = Prepare { cmd };
//                 let message = MessageType::Prepare(prepare);
//                 let data = ConsensusMessage { msg: message };
//                 let msg = coder::serialize_into_bytes(&data);
//                 let topic = IdentTopic::new("Consensus");
//                 if let Err(e) = self
//                     .peer_mut()
//                     .network_swarm_mut()
//                     .behaviour_mut()
//                     .gossipsub
//                     .publish(topic.clone(), msg)
//                 {
//                     eprintln!("Publish message error:{:?}", e);
//                 }
//                 println!("Broadcast Prepare message success!");
//             }
//             MessageType::Prepare(prepare) => {
//                 let cmd = prepare.cmd;
//                 println!("Consensus node received Preare message : {:?}", cmd);
//                 let commit = Commit { cmd };
//                 let message = MessageType::Commit(commit);
//                 let data = ConsensusMessage { msg: message };
//                 let msg = coder::serialize_into_bytes(&data);
//                 let topic = IdentTopic::new("Consensus");
//                 if let Err(e) = self
//                     .peer_mut()
//                     .network_swarm_mut()
//                     .behaviour_mut()
//                     .gossipsub
//                     .publish(topic.clone(), msg)
//                 {
//                     eprintln!("Publish message error:{:?}", e);
//                 }
//                 println!("Broadcast Commit message success!");
//             }
//             MessageType::Commit(commit) => {
//                 let cmd = commit.cmd;
//                 println!("Consensus node received Commit message : {:?}", cmd);
//                 let reply = Reply { cmd };
//                 let message = MessageType::Reply(reply);
//                 let data = ConsensusMessage { msg: message };
//                 let msg = coder::serialize_into_bytes(&data);
//                 let topic = IdentTopic::new("Consensus");
//                 if let Err(e) = self
//                     .peer_mut()
//                     .network_swarm_mut()
//                     .behaviour_mut()
//                     .gossipsub
//                     .publish(topic.clone(), msg)
//                 {
//                     eprintln!("Publish message error:{:?}", e);
//                 }
//                 println!("Broadcast Reply message success!");
//                 self.reset_count();
//             }
//             MessageType::Reply(reply) => {
//                 // if self.peer_id().to_string().as_str() == self.leader_id().as_str() {
//                 println!("Broadcast Reply********");
//                 let request = Request { cmd: reply.cmd };
//                 return ConsensusEnd::Yes(request);
//                 // }
//             }
//         }

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
