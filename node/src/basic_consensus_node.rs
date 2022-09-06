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
        ConsensusStartData, InteractiveMessage, MaliciousAction, Message, Request, Component,
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
    controller_id: Option<PeerId>,
    analyzer_id: Option<PeerId>,
    // executor: TExecutor,
    request_buffer: HashMap<String, Request>,
    log: TLog,
    state: TState,

    // notifier
    consensus_notify: Arc<Notify>,
    view_timeout_notify: Arc<Notify>,
    protocol_stop_notify: Arc<Notify>,

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
    pub fn new(peer: Peer, port: &str) -> Self {
        // let db_path = format!("./storage/data/{}_public_keys", port);
        Self {
            peer,
            controller_id: None,
            analyzer_id: None,
            request_buffer: HashMap::new(),
            log: TLog::default(),
            state: TState::default(),
            consensus_notify: Arc::new(Notify::new()),
            view_timeout_notify: Arc::new(Notify::new()),
            protocol_stop_notify: Arc::new(Notify::new()),

            diagnostic_indicators: Vec::new(),
            mode: ConsensusNodeMode::Uninitialized,
        }
    }

    pub async fn network_start(&mut self) -> Result<(), Box<dyn Error>> {
        self.peer_mut().swarm_start(true).await?;

        self.protocol_start_listening().await;
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

    pub fn protocol_stop_notify(&self) -> Arc<Notify> {
        self.protocol_stop_notify.clone()
    }

    pub fn controller_id(&self) -> PeerId {
        self.controller_id.expect("Not found controller!")
    }

    pub fn analyzer_id(&self) -> PeerId {
        self.analyzer_id.expect("Not found analyzer")
    }

    pub fn add_controller(&mut self, controller_id: PeerId) {
        self.controller_id = Some(controller_id);
    }

    pub fn add_analyzer(&mut self, analyzer_id: PeerId) {
        self.analyzer_id = Some(analyzer_id);
    }

    pub fn write_requests_into_buffer(&mut self, requests: Vec<Request>) {
        for request in requests {
            let request_hash = get_request_hash(&request);
            self.request_buffer.insert(request_hash, request);
        }
    }

    pub fn verify_initialization(&self) -> bool {
        matches!(self.controller_id, Some(_)) && matches!(self.analyzer_id, Some(_))
    }

    pub fn reset(&mut self) {
        self.request_buffer.clear();
    }

    // The consensus's handler on the controller message
    pub fn controller_message_handler(&mut self, message: Message) {
        match message.interactive_message {
            InteractiveMessage::SubscribeConsensusTopic => {
                let topic = IdentTopic::new("Consensus");
                if let Err(e) = self
                    .peer_mut()
                    .network_swarm_mut()
                    .behaviour_mut()
                    .gossipsub
                    .subscribe(&topic)
                {
                    eprintln!("Subscribe consensus topic error:{:?}", e);
                };

                let interactive_message = InteractiveMessage::SubscribeConsensusTopicSuccess;
                let message = Message {
                    interactive_message,
                };

                let controller_id = self.controller_id();
                let serialized_message = coder::serialize_into_bytes(&message);
                self.peer_mut()
                    .network_swarm_mut()
                    .behaviour_mut()
                    .unicast
                    .send_message(&controller_id, serialized_message);
            }
            _ => {}
        };
    }

    pub async fn protocol_start_listening(&mut self) {
        loop {
            tokio::select! {
                    event = self.peer_mut().network_swarm_mut().select_next_some() => match event {
                        SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                            // println!("tblskey.");
                            // self.consensus_protocol_message_handler(&message.data);
                        }
                        SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                            propagation_source: peer_id,
                            message_id: _id,
                            message,
                        })) => {
                            let message: Message = coder::deserialize_for_bytes(&message.data);
                            match message.interactive_message {
                                InteractiveMessage::ComponentInfo(Component::Controller(id_bytes)) => {
                                    let controller_id = PeerId::from_bytes(&id_bytes[..]).unwrap();
                                    println!("Controller PeerId: {:?}", controller_id.to_string());
                                    self.add_controller(controller_id);
                                }
                                InteractiveMessage::ComponentInfo(Component::Analyzer(id_bytes)) => {
                                    let controller_id = PeerId::from_bytes(&id_bytes[..]).unwrap();
                                    println!("Controller PeerId: {:?}", controller_id.to_string());
                                    self.add_analyzer(controller_id);
                                }
                                InteractiveMessage::ProtocolStart => {
                                    self.verify_initialization();
                                    self.message_handler_start().await;
                                }
                                InteractiveMessage::Reset => {
                                    self.verify_initialization();
                                    self.protocol_stop_notify().notify_one();
                                }
                                _ => {}
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

    pub async fn message_handler_start(&mut self) {
        let consensus_notify = self.consensus_notify();
        let view_timeout_notify = self.view_timeout_notify();
        let protocol_stop_notify = self.protocol_stop_notify();

        // Kick it off
        loop {
            tokio::select! {
                _ = protocol_stop_notify.notified() => {
                    self.reset();
                    break;
                }
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
                    let analyzer_id = self.analyzer_id();
                    self.peer_mut().network_swarm_mut().behaviour_mut().unicast.send_message(&analyzer_id, serialized_consensus_data);
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
                        if peer_id.to_string().eq(&self.controller_id().to_string()) {
                            let message: Message = coder::deserialize_for_bytes(&message.data);
                            self.controller_message_handler(message);
                            // let msg: CommandMessage = coder::deserialize_for_bytes(&message.data);
                            // match msg.command {
                            //     Command::MakeConsensusRequests(requests) => {
                            //         self.write_requests_into_buffer(requests);
                            //     }
                            //     _ => {}
                            // }
                        } else {
                            let is_end = self.consensus_protocol_message_handler(&message.data);
                            if let ConsensusEnd::Yes(request) = is_end {
                                let consensus_end_data = ConsensusEndData { request, completed_time: Local::now().timestamp_millis() };
                                let data = ConsensusData::ConsensusEndData(consensus_end_data);
                                self.push_consensus_data_to_analysis_node(&data);
                            }
                        }
                    }
                    // SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                    //     let swarm = self.peer_mut().network_swarm_mut();
                    //     for (peer, _) in list {
                    //         println!("Discovered {:?}", &peer);
                    //         swarm.behaviour_mut().unicast.add_node_to_partial_view(&peer);
                    //         swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                    //         // self.connected_nodes.insert(peer.to_string(), peer.clone());
                    //     }
                    //     //println!("Connected_nodes: {:?}", self.connected_nodes.lock().await);
                    // }
                    // SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Expired(list))) => {
                    //     let swarm = self.peer_mut().network_swarm_mut();
                    //     for (peer, _) in list {
                    //         if !swarm.behaviour_mut().mdns.has_node(&peer) {
                    //             swarm.behaviour_mut().unicast.remove_node_from_partial_view(&peer);
                    //             swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer);
                    //             // self.connected_nodes.remove(&peer.to_string());
                    //         }
                    //     }
                    // }
                    // SwarmEvent::NewListenAddr { address, .. } => {
                    //     println!("Listening on {:?}", address);
                    // }
                    _ => {}
                }
            }
        }
    }
}

enum ConsensusNodeMode {
    Uninitialized,
    Honest,
    Dishonest(MaliciousAction),
    Outage(usize),
}
