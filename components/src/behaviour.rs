use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};
use libp2p::PeerId;
use tokio::sync::Notify;
use crate::message::Request;

pub trait ProtocolBehaviour {


    // sada
    //
    //

    ///
    ///   init the protocol's timeout notify by the param (timeout_notify)
    ///
    fn init_timeout_notify(&mut self, timeout_notify: Arc<Notify>) {}


    ///
    ///     In addition to the default network startup and key distribution.
    ///     the protocol may have additional initiators that can be added to this method.
    ///  
    fn extra_initial_start(
        &mut self,
        consensus_nodes: HashSet<PeerId>,
        current_peer_id: Vec<u8>,
    ) -> PhaseState {
        let mut queue = VecDeque::new();
        queue.push_back(SendType::Broadcast(vec![]));
        PhaseState::ContinueExecute(queue)
    }


    ///
    ///     Receives a consensus request from the Controller node
    ///  
    fn receive_consensus_requests(&mut self, requests: Vec<Request>);


    ///
    ///     Consensus protocol's message handler,it handles the logic of the consensus protocol.
    ///  
    fn consensus_protocol_message_handler(&mut self, _msg: &[u8],current_peer_id: Vec<u8>,
        peer_id: Option<PeerId>) -> PhaseState {
        let mut queue = VecDeque::new();
        queue.push_back(SendType::Broadcast(vec![]));
        PhaseState::ContinueExecute(queue)
    }


    /// 
    /// Get the phase_num the consensus protocol is processing in.
    ///  
    fn get_current_phase(&mut self, _msg: &[u8]) -> u8 {1}


    ///
    ///     Reset all the running data of the consensus protocol.
    ///  
    fn protocol_reset(&mut self) {}


    ///
    ///     View_timeout_handler contains what to do when a view times out. 
    ///
    fn view_timeout_handler(&mut self,current_peer_id: PeerId) -> PhaseState {
        let mut queue = VecDeque::new();
        queue.push_back(SendType::Broadcast(vec![]));
        PhaseState::ContinueExecute(queue)
    }


    ///
    ///     It serializes the consensus data structure and generate a map fron phase_num(u8) to serialized data.
    ///  
    fn protocol_phases(&mut self) -> HashMap<u8, Vec<u8>> {
        println!("No security test interface is implemented！");
        HashMap::new()
    }


    ///
    ///     It stores the map from phase_num(u8) to phase_name(String).
    ///
    fn phase_map(&self) -> HashMap<u8,String> {
        HashMap::new()
    }


    ///
    ///     It gets the current request the consensus protocol round is processing.
    ///
    fn current_request(&self) -> Request;


    ///
    ///     It checks whether this node is the leader.
    ///
    fn is_leader(&self, current_peer_id: Vec<u8>) -> bool {
        false
    }


    ///
    ///     It sets which node is the leader.
    ///
    fn set_leader(&mut self, is_leader: bool) {
        
    }


    ///
    ///     It serializes the request data structure.
    ///
    fn generate_serialized_request_message(&self, request: &Request) -> Vec<u8> {
        vec![]
    }


    ///
    ///     It checks whether the given request has been taken to the consensus process.
    ///
    fn check_taken_request(&self,request:Vec<u8>) -> bool {
        false
    }


    ///
    ///     It sets the request.
    ///
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
