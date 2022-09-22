use std::{collections::HashMap, error::Error, sync::Arc, time::Duration};

use crate::{
    behaviour::{
        ConsensusNodeBehaviour, NodeStateUpdateBehaviour, PhaseState, ProtocolLogsReadBehaviour,
        SendType,
    },
    common::get_request_hash,
    message::{
        Component, ConsensusData, ConsensusDataMessage, ConsensusEndData, ConsensusStartData,
        InteractiveMessage, Message, Request,
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

use serde::{Deserialize, Serialize};
use tokio::{
    sync::Notify,
    time::{interval_at, Instant},
};
use utils::coder::{self};

pub struct ConsensusNode<TLog, TState, TProtocol>
where
    TLog: Default + ProtocolLogsReadBehaviour,
    TState: Default + NodeStateUpdateBehaviour,
    TProtocol: Default + ConsensusNodeBehaviour,
{
    peer: Peer,
    controller_id: Option<PeerId>,
    analyzer_id: Option<PeerId>,
    // executor: TExecutor,
    request_buffer: HashMap<String, Request>,
    log: TLog,
    state: TState,
    protocol: TProtocol,

    // notifier
    consensus_notify: Arc<Notify>,
    view_timeout_notify: Arc<Notify>,
    protocol_stop_notify: Arc<Notify>,
    crash_notify: Arc<Notify>,

    // consensus node's mode: Honest、Dishonest、Outage
    mode: ConsensusNodeMode,
}

impl<TLog, TState, TProtocol> ConsensusNode<TLog, TState, TProtocol>
where
    TLog: Default + ProtocolLogsReadBehaviour,
    TState: Default + NodeStateUpdateBehaviour,
    TProtocol: Default + ConsensusNodeBehaviour,
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
            protocol: TProtocol::default(),
            consensus_notify: Arc::new(Notify::new()),
            view_timeout_notify: Arc::new(Notify::new()),
            protocol_stop_notify: Arc::new(Notify::new()),
            crash_notify: Arc::new(Notify::new()),

            mode: ConsensusNodeMode::Uninitialized,
        }
    }

    pub async fn network_start(&mut self) -> Result<(), Box<dyn Error>> {
        self.peer_mut().swarm_start(true).await?;

        self.subscribe_topics();
        self.protocol_start_listening().await;
        // self.executor.proposal_state_check();
        // self.message_handler_start().await;
        Ok(())
    }

    // subscribe gossip topics
    pub fn subscribe_topics(&mut self) {
        let topic_1 = IdentTopic::new("Initialization");
        if let Err(e) = self
            .peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic_1)
        {
            eprintln!("Subscribe error:{:?}", e);
        };

        let topic_2 = IdentTopic::new("Reset");
        if let Err(e) = self
            .peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic_2)
        {
            eprintln!("Subscribe error:{:?}", e);
        };
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

    pub fn protocol_mut(&mut self) -> &mut TProtocol {
        &mut self.protocol
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

    pub fn mode(&self) -> ConsensusNodeMode {
        self.mode.clone()
    }

    pub fn crash_notify(&self) -> Arc<Notify> {
        self.crash_notify.clone()
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

    pub fn set_mode(&mut self, mode: ConsensusNodeMode) {
        self.mode = mode;
    }

    pub fn protocol_start_preprocess(&mut self) {
        let mode = self.mode();
        match mode {
            ConsensusNodeMode::Uninitialized => {}
            ConsensusNodeMode::Honest(_) => {
                println!("I'm a honest consensus node!");
            }
            ConsensusNodeMode::Dishonest(m) => {
                println!("I'm a dishonest consensus node! [{:?}]", m);
            }
            ConsensusNodeMode::Crash(internal) => {
                println!("I'm a crash consensus node in future!");
                self.crash_timer_start(internal);
            }
        }
    }

    pub fn reset(&mut self, mode: ConsensusNodeMode) {
        self.request_buffer.clear();

        let interactive_message = InteractiveMessage::ResetSuccess(mode);
        let message = Message {
            interactive_message,
            source: vec![],
        };

        let controller_id = self.controller_id();
        let serialized_message = coder::serialize_into_bytes(&message);
        self.peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .unicast
            .send_message(&controller_id, serialized_message);
    }

    pub fn crash_timer_start(&mut self, internal: u64) {
        println!("\nCrash timer start...");
        let duration = Duration::from_millis(internal);
        // println!("Current view timeout: {}", self.state.current_view_timeout);
        let crash_notify = self.crash_notify();

        tokio::spawn(async move {
            let start = Instant::now() + duration;
            let mut intv = interval_at(start, duration);

            intv.tick().await;
            crash_notify.notify_one();
        });
    }

    // The consensus's handler on the controller message
    pub fn running_controller_message_handler(&mut self, message: Message) {
        match message.interactive_message {
            InteractiveMessage::Reset(_) => {
                println!("Reset!!!");
                if self.verify_initialization() {
                    self.protocol_stop_notify().notify_one();
                } else {
                    eprintln!("Not found initialization!");
                };
            }
            InteractiveMessage::ConsensusNodeMode(_) => {
                println!("sbsbsbsbsbsbs");
            }
            _ => {}
        };
    }

    pub async fn no_running_controller_message_handler(&mut self, message: Message) {
        match message.interactive_message {
            InteractiveMessage::ConfigureConsensusNode(state) => {

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

                self.set_mode(ConsensusNodeMode::Honest(state));

                // println!("Honest Mode!!!!{:?}", self.mode());

                let interactive_message = InteractiveMessage::ConsensusNodeModeSuccess(self.mode());
                let message = Message {
                    interactive_message,
                    source: vec![],
                };

                let controller_id = self.controller_id();
                let serialized_message = coder::serialize_into_bytes(&message);
                self.peer_mut()
                    .network_swarm_mut()
                    .behaviour_mut()
                    .unicast
                    .send_message(&controller_id, serialized_message);
            }
            InteractiveMessage::ProtocolStart(_) => {
                println!("######################################");
                println!("Protocol Start!!!");
                self.verify_initialization();
                self.message_handler_start().await;
            }
            InteractiveMessage::JoinConsensus(_) => {
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

                self.set_mode(ConsensusNodeMode::Honest(ConfigureState::Other));

                let interactive_message = InteractiveMessage::JoinConsensusSuccess;
                let message = Message {
                    interactive_message,
                    source: vec![],
                };

                let controller_id = self.controller_id();
                let serialized_message = coder::serialize_into_bytes(&message);
                self.peer_mut()
                    .network_swarm_mut()
                    .behaviour_mut()
                    .unicast
                    .send_message(&controller_id, serialized_message);
            }
            InteractiveMessage::ConsensusNodeMode(mode) => {
                if !self.mode().eq(&mode) {
                    self.set_mode(mode);
                    println!("Set mode success: {:?}", self.mode());

                    let interactive_message =
                        InteractiveMessage::ConsensusNodeModeSuccess(self.mode());
                    let message = Message {
                        interactive_message,
                        source: vec![],
                    };

                    let controller_id = self.controller_id();
                    let serialized_message = coder::serialize_into_bytes(&message);
                    self.peer_mut()
                        .network_swarm_mut()
                        .behaviour_mut()
                        .unicast
                        .send_message(&controller_id, serialized_message);
                }
            }
            _ => {}
        };
    }

    pub async fn protocol_start_listening(&mut self) {
        loop {
            tokio::select! {
                    event = self.peer_mut().network_swarm_mut().select_next_some() => match event {
                        SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                            println!("Controller unicast!");
                            let peer_id = PeerId::from_bytes(&message.source[..]).unwrap();
                            if peer_id.to_string().eq(&self.controller_id().to_string()) {
                                let message: Message = coder::deserialize_for_bytes(&message.data);
                                self.no_running_controller_message_handler(message).await;
                            };
                        }
                        SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                            propagation_source: _peer_id,
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
                                    println!("Analyzer PeerId: {:?}", controller_id.to_string());
                                    self.add_analyzer(controller_id);
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

    pub fn send_message(&mut self, send_type: SendType) {
        match send_type {
            SendType::Broadcast(msg) => {
                let topic = IdentTopic::new("Consensus");
                if let Err(e) = self
                    .peer_mut()
                    .network_swarm_mut()
                    .behaviour_mut()
                    .gossipsub
                    .publish(topic, msg)
                {
                    eprintln!("Publish message error:{:?}", e);
                }
            },
            SendType::Unicast(receiver, msg) => {
                self.peer_mut()
                    .network_swarm_mut()
                    .behaviour_mut()
                    .unicast
                    .send_message(&receiver, msg);
            },
        }
    }

    pub fn malicious_trigger(&mut self, phase: u8, send_type: SendType) {
        match self.mode() {
            ConsensusNodeMode::Dishonest(MaliciousMode::LeaderFeignDeath(_, p)) => {
                if phase == p {

                }
            },
            ConsensusNodeMode::Dishonest(MaliciousMode::LeaderSendDuplicateMessages(_, p)) => {
                if phase == p {
                    self.send_message(send_type);
                }
            }
            ConsensusNodeMode::Dishonest(MaliciousMode::LeaderSendAmbiguousMessage(_, p, _)) => {
                if phase == p {

                }
            }
            ConsensusNodeMode::Dishonest(MaliciousMode::LeaderDelaySendMessage(_, p)) => {
                if phase == p {

                }
            },
            ConsensusNodeMode::Dishonest(MaliciousMode::ReplicaNodeConspireForgeMessages(_, p)) => {
                if phase == p {

                }
            }
            _ => {},
        }
    }

    pub fn check_protocol_phase_state(&mut self, phase_state: PhaseState) {
        match phase_state {
            PhaseState::Over(request) => {
                let consensus_end_data = ConsensusEndData {
                    request,
                    completed_time: Local::now().timestamp_millis(),
                };
                let data = ConsensusData::ConsensusEndData(consensus_end_data);
                let consensus_data_message = ConsensusDataMessage { data };

                let serialized_consensus_data_message =
                    coder::serialize_into_bytes(&consensus_data_message);

                let analysis_node_id = self.analyzer_id();
                self.peer_mut()
                    .network_swarm_mut()
                    .behaviour_mut()
                    .unicast
                    .send_message(&analysis_node_id, serialized_consensus_data_message);
            }
            PhaseState::ContinueExecute(SendType::Broadcast(msg)) => {
                let phase = self.protocol_mut().get_current_phase(&msg[..]);
                let send_type = SendType::Broadcast(msg);
                self.malicious_trigger(phase, send_type.clone());
                self.send_message(send_type);
            }
            PhaseState::ContinueExecute(SendType::Unicast(r, msg)) => {
                let phase = self.protocol_mut().get_current_phase(&msg[..]);
                let send_type = SendType::Unicast(r, msg);
                self.malicious_trigger(phase, send_type.clone());
                self.send_message(send_type);
            }
        }
    }

    pub async fn message_handler_start(&mut self) {
        let consensus_notify = self.consensus_notify();
        let view_timeout_notify = self.view_timeout_notify();
        let protocol_stop_notify = self.protocol_stop_notify();
        let crash_notify = self.crash_notify();

        self.protocol_start_preprocess();
        // Kick it off
        loop {
            tokio::select! {
                _ = protocol_stop_notify.notified() => {
                    let mode = self.mode();
                    self.reset(mode);
                    println!("Reset completed!!!");
                    break;
                }
                _ = crash_notify.notified() => {
                    let topic = IdentTopic::new("Consensus");
                    if let Err(e) = self.peer_mut().network_swarm_mut().behaviour_mut().gossipsub.unsubscribe(&topic) {
                        eprintln!("Unsubscribe consensus topic error:{:?}", e);
                    };

                    println!("Crash!!!!!!!!!!");

                    let mode = self.mode();
                    println!("Mode:{:?}", &mode);
                    self.reset(mode);
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
                    let phase_state = self.protocol_mut().consensus_protocol_message_handler(&serialized_request_message);
                    self.check_protocol_phase_state(phase_state);
                },
                _ = view_timeout_notify.notified() => {
                    self.protocol_mut().view_timeout_handler();
                },
                event = self.peer_mut().network_swarm_mut().select_next_some() => match event {
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        // println!("tblskey.");
                        let peer_id = PeerId::from_bytes(&message.source[..]).unwrap();
                        if peer_id.to_string().eq(&self.controller_id().to_string()) {
                            let message: Message = coder::deserialize_for_bytes(&message.data);
                            self.running_controller_message_handler(message);
                        } else {
                            let phase_state = self.protocol_mut().consensus_protocol_message_handler(&message.data);
                            self.check_protocol_phase_state(phase_state);
                        };
                    }
                    SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                        propagation_source: _peer_id,
                        message_id: _id,
                        message,
                    })) => {
                        let phase_state = self.protocol_mut().consensus_protocol_message_handler(&message.data);
                        self.check_protocol_phase_state(phase_state);
                    }
                    _ => {
                        println!("Default!");
                    }
                }
            }
        }
    }
}

pub type TriggerTimeInterval = u64;
pub type ProtocolPhase = u8;
pub type NumberOfAmbiguousMessage = u16;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ConsensusNodeMode {
    Uninitialized,
    Honest(ConfigureState),
    Dishonest(MaliciousMode),
    Crash(TriggerTimeInterval),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum MaliciousMode {
    LeaderFeignDeath(TriggerTimeInterval, ProtocolPhase),
    LeaderSendAmbiguousMessage(TriggerTimeInterval, ProtocolPhase, NumberOfAmbiguousMessage),
    LeaderDelaySendMessage(TriggerTimeInterval, ProtocolPhase),
    LeaderSendDuplicateMessages(TriggerTimeInterval, ProtocolPhase),

    ReplicaNodeConspireForgeMessages(TriggerTimeInterval, ProtocolPhase),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ConfigureState {
    First,
    Other,
}
