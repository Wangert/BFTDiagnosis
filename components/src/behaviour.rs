use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use libp2p::PeerId;
use network::peer::Peer;
use tokio::sync::Notify;

use crate::message::{ConsensusData, Request};

pub trait ProtocolBehaviour {
    fn init_timeout_notify(&mut self, timeout_notify: Arc<Notify>) {}
    // In addition to the default network startup,
    // the protocol may have additional initiators that can be added to this method.
    fn extra_initial_start(
        &mut self,
        consensus_nodes: HashSet<PeerId>,
        current_peer_id: Vec<u8>,
        analyzer_id: String,
    ) -> PhaseState {
        let mut queue = VecDeque::new();
        queue.push_back(SendType::Broadcast(vec![]));
        PhaseState::ContinueExecute(queue)
    }
    // Receives a consensus request from the Controller node
    fn receive_consensus_requests(&mut self, requests: Vec<Request>);
    // Consensus protocol's message handler
    fn consensus_protocol_message_handler(&mut self, _msg: &[u8],current_peer_id: Vec<u8>,
        peer_id: Option<PeerId>) -> PhaseState {
        let mut queue = VecDeque::new();
        queue.push_back(SendType::Broadcast(vec![]));
        PhaseState::ContinueExecute(queue)
    }
    fn get_current_phase(&mut self, _msg: &[u8]) -> u8 {
        1
    }

    fn protocol_reset(&mut self) {

    }
    // When a view times out, a series of operations are performed, such as view change
    fn view_timeout_handler(&mut self,current_peer_id: PeerId) -> PhaseState {
        let mut queue = VecDeque::new();
        queue.push_back(SendType::Broadcast(vec![]));
        PhaseState::ContinueExecute(queue)
    }

    fn protocol_phases(&mut self) -> HashMap<u8, Vec<u8>> {
        println!("No security test interface is implemented！");
        HashMap::new()
    }

    fn phase_map(&self) -> HashMap<u8,String> {
        HashMap::new()
    }

    fn current_request(&self) -> Request;

    fn is_leader(&self, current_peer_id: Vec<u8>) -> bool {
        false
    }

    fn set_leader(&mut self, is_leader: bool) {
        
    }

    fn generate_serialized_request_message(&self, request: &Request) -> Vec<u8> {
        vec![]
    }

    fn check_taken_request(&self,request:Vec<u8>) -> bool {
        false
    }

    fn set_current_request(&mut self, request: &Request) {}
}

type MessageBytes = Vec<u8>;

#[derive(Debug, Clone)]
pub enum PhaseState {
    Over(Option<Request>),
    OverMessage(Option<Request>,VecDeque<SendType>),
    ContinueExecute(VecDeque<SendType>),
    Complete(Request, VecDeque<SendType>),
}

#[derive(Debug, Clone)]
pub enum SendType {
    Broadcast(MessageBytes),
    Unicast(PeerId, MessageBytes),
    AmbiguousBroadcast(MessageBytes, MessageBytes, u16),
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
