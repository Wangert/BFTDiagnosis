use std::{collections::HashMap, error::Error, hash::Hash, sync::Arc};

use crate::{
    behaviour::{
        ConsensusEnd, ConsensusNodeBehaviour, NodeStateUpdateBehaviour, ProtocolHandler,
        ProtocolLogsReadBehaviour,
    },
    common::get_request_hash,
    config::ExtraInfo,
    message::{
        Command, CommandMessage, ConsensusData, ConsensusDataMessage, ConsensusEndData,
        ConsensusStartData, Request, MaliciousAction,
    },
};

use chrono::Local;
use libp2p::{
    futures::StreamExt,
    gossipsub::{GossipsubEvent, IdentTopic},
    mdns::MdnsEvent,
    swarm::SwarmEvent,
    PeerId,
};
use network::{
    p2p_protocols::{base_behaviour::OutEvent, unicast::behaviour::UnicastEvent},
    peer::Peer,
};

use tokio::sync::Notify;
use utils::coder::{self};

pub struct ConsensusNode<TLog, TState>
where
    TLog: Default + ProtocolLogsReadBehaviour,
    TState: Default + NodeStateUpdateBehaviour,
{
    peer: Peer,
    controller_node_id: PeerId,
    analysis_node_id: PeerId,
    // executor: TExecutor,
    request_buffer: HashMap<String, Request>,
    log: TLog,
    state: TState,

    // notifier
    consensus_notify: Arc<Notify>,
    view_timeout_notify: Arc<Notify>,

    // diagnostic indicators
    diagnostic_indicators: Vec<Command>,

    // consensus node's mode: Honest、Dishonest、Outage
    mode: ConsensusNodeMode,
}

impl<TLog, TState> ConsensusNode<TLog, TState>
where
    TLog: Default + ProtocolLogsReadBehaviour,
    TState: Default + NodeStateUpdateBehaviour,
{
    pub fn new(peer: Peer, extra_info: &ExtraInfo, port: &str) -> Self {
        // let db_path = format!("./storage/data/{}_public_keys", port);
        Self {
            peer,
            controller_node_id: extra_info.controller_node_id,
            analysis_node_id: extra_info.analysis_node_id,
            request_buffer: HashMap::new(),
            log: TLog::default(),
            state: TState::default(),
            consensus_notify: Arc::new(Notify::new()),
            view_timeout_notify: Arc::new(Notify::new()),
            diagnostic_indicators: Vec::new(),
            mode: ConsensusNodeMode::Honest,
        }
    }

    pub async fn network_start(&mut self) -> Result<(), Box<dyn Error>> {
        self.peer_mut().swarm_start(true).await?;

        // self.executor.proposal_state_check();
        // self.message_handler_start().await;

        Ok(())
    }

    pub fn peer_mut(&mut self) -> &mut Peer {
        &mut self.peer
    }

    pub fn log_mut(&mut self) -> &mut TLog {
        &mut self.log
    }

    pub fn state_mut(&mut self) -> &mut TState {
        &mut self.state
    }

    pub fn peer_id(&self) -> &PeerId {
        &self.peer.id
    }

    pub fn next_request(&self) -> Option<Request> {
        if self.request_buffer.is_empty() {
            return None;
        }
        let mut request_buffer_iter = self.request_buffer.iter();
        let (_, next_request) = request_buffer_iter.next().unwrap();

        Some(next_request.clone())
    }

    pub fn consensus_notify(&self) -> Arc<Notify> {
        self.consensus_notify.clone()
    }

    pub fn view_timeout_notify(&self) -> Arc<Notify> {
        self.view_timeout_notify.clone()
    }

    pub fn analysis_node_id(&self) -> PeerId {
        self.analysis_node_id.clone()
    }

    pub fn write_requests_into_buffer(&mut self, requests: Vec<Request>) {
        for request in requests {
            let request_hash = get_request_hash(&request);
            self.request_buffer.insert(request_hash, request);
        }
    }

    pub async fn message_handler_start(&mut self) {
        let consensus_notify = self.consensus_notify();
        let view_timeout_notify = self.view_timeout_notify();

        // Kick it off
        loop {
            tokio::select! {
                // start consensus notify
                _ = consensus_notify.notified() => {
                    // get a request in buffer
                    let request = self.next_request();
                    if let None = request {
                        continue;
                    }

                    let request = request.unwrap();
                    let serialized_request_message = coder::serialize_into_bytes(&request);

                    let consensus_start_data = ConsensusStartData { request, start_time: Local::now().timestamp_millis() };
                    let consensus_data_message = ConsensusDataMessage {
                        data: ConsensusData::ConsensusStartData(consensus_start_data),
                    };
                    let serialized_consensus_data = coder::serialize_into_bytes(&consensus_data_message);
                    let analysis_node_id = self.analysis_node_id.clone();
                    self.peer_mut().network_swarm_mut().behaviour_mut().unicast.send_message(&analysis_node_id, serialized_consensus_data);
                    // handle request
                    self.consensus_protocol_message_handler(&serialized_request_message);
                },
                _ = view_timeout_notify.notified() => {
                    self.view_timeout_handler();
                },
                event = self.peer_mut().network_swarm_mut().select_next_some() => match event {
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        // println!("tblskey.");
                        self.consensus_protocol_message_handler(&message.data);
                    }
                    SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                        propagation_source: peer_id,
                        message_id: _id,
                        message,
                    })) => {
                        if peer_id.to_string().eq(&self.controller_node_id.to_string()) {
                            let msg: CommandMessage = coder::deserialize_for_bytes(&message.data);
                            match msg.command {
                                Command::MakeConsensusRequests(requests) => {
                                    self.write_requests_into_buffer(requests);
                                }
                                _ => {}
                            }
                        } else {
                            let is_end = self.consensus_protocol_message_handler(&message.data);
                            if let ConsensusEnd::Yes(request) = is_end {
                                let consensus_end_data = ConsensusEndData { request, completed_time: Local::now().timestamp_millis() };
                                let data = ConsensusData::ConsensusEndData(consensus_end_data);
                                self.push_consensus_data_to_analysis_node(&data);
                            }
                        }
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                        let swarm = self.peer_mut().network_swarm_mut();
                        for (peer, _) in list {
                            println!("Discovered {:?}", &peer);
                            swarm.behaviour_mut().unicast.add_node_to_partial_view(&peer);
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                            // self.connected_nodes.insert(peer.to_string(), peer.clone());
                        }
                        //println!("Connected_nodes: {:?}", self.connected_nodes.lock().await);
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Expired(list))) => {
                        let swarm = self.peer_mut().network_swarm_mut();
                        for (peer, _) in list {
                            if !swarm.behaviour_mut().mdns.has_node(&peer) {
                                swarm.behaviour_mut().unicast.remove_node_from_partial_view(&peer);
                                swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer);
                                // self.connected_nodes.remove(&peer.to_string());
                            }
                        }
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on {:?}", address);
                    }
                    _ => {}
                }
            }
        }
    }

    // pub fn handle_test(&mut self, request: &Request, current_peer_id: &[u8]) {
    //     let msg = Message {
    //         command: Command::MakeAConsensusRequest(request.clone()),
    //     };
    //     let serialized_msg = coder::serialize_into_bytes(&msg);

    //     let topic = IdentTopic::new("consensus");
    //     if let Err(e) = self
    //         .peer_mut()
    //         .network_swarm_mut()
    //         .behaviour_mut()
    //         .gossipsub
    //         .publish(topic.clone(), serialized_msg)
    //     {
    //         eprintln!("Publish message error:{:?}", e);
    //     }
    // }

    // pub fn consensus_protocol_message_handler(&mut self, request: &Request, current_peer_id: &[u8]) {
    //     self.handle_test(request, current_peer_id)
    // }
}

enum ConsensusNodeMode {
    Honest,
    Dishonest(MaliciousAction),
    Outage(usize),
}
