use std::sync::Arc;

use libp2p::PeerId;
use network::peer::Peer;
use tokio::sync::Notify;

use crate::message::{ConsensusData, Request};

pub trait ConsensusNodeBehaviour {
    // In addition to the default network startup,
    // the protocol may have additional initiators that can be added to this method.
    fn extra_initial_start(&mut self) {}
    // Receives a consensus request from the Controller node
    fn receive_consensus_requests(&mut self, requests: Vec<Request>);
    // Consensus protocol's message handler
    fn consensus_protocol_message_handler(&mut self, _msg: &[u8]) -> PhaseState {
        PhaseState::ContinueExecute(SendType::Broadcast(vec![]))
    }
    fn get_current_phase(&mut self, _msg: &[u8]) -> u8 {
        1
    }
    // When a view times out, a series of operations are performed, such as view change
    fn view_timeout_handler(&mut self) {}
    // When the Analysis Node reads process data from a consensus node,
    // it handle a poll request from the Analysis Node.
    fn analysis_node_poll_request(&mut self);
    // When a poll request is processed by current consensus node,
    // push process datas to the Analysis Node.
    fn push_consensus_data_to_analysis_node(&mut self, _: &ConsensusData);

}

type MessageBytes = Vec<u8>;

#[derive(Debug, Clone)]
pub enum PhaseState {
    Over(Request),
    ContinueExecute(SendType),
}

#[derive(Debug, Clone)]
pub enum SendType {
    Broadcast(MessageBytes),
    Unicast(PeerId, MessageBytes),
}

// pub trait  {
    
// }

pub trait ProtocolHandler {
    // how to handle messages during the execution of consensus protocols
    fn message_handler(&mut self, msg: &[u8], _current_node_id: &[u8]);

    //
    fn consensus_notify(&self) -> Arc<Notify>;
    fn view_timeout_notify(&self) -> Arc<Notify>;
}

pub trait ProtocolLogsReadBehaviour {
    fn get_ledger(&mut self);
    fn get_current_leader(&self);
}

pub trait NodeStateUpdateBehaviour {
    fn update_consensus_node_count(&mut self, _count: usize);
}
