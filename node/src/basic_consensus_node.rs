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
use serde_json::{json, Map, Value};
use tokio::{
    sync::{Mutex, Notify},
    time::{interval_at, Instant},
};
use utils::{
    coder::{self, serialize_into_json_bytes, serialize_into_json_str},
    parse::motify_map_value_with_field,
};

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
    malicious_trigger_notify: Arc<Notify>,

    malicious_flag: bool,
    is_leader: bool,
    other_consensus_node: Vec<PeerId>,

    // consensus node's mode: Honest、Dishonest、Outage
    mode: ConsensusNodeMode,
    consensus_state: Arc<Mutex<bool>>,

    protocol_running: bool,
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
            malicious_trigger_notify: Arc::new(Notify::new()),
            crash_notify: Arc::new(Notify::new()),

            malicious_flag: false,
            is_leader: false,

            other_consensus_node: vec![],

            mode: ConsensusNodeMode::Uninitialized,
            consensus_state: Arc::new(Mutex::new(false)),

            protocol_running: false,
        }
    }

    pub async fn network_start(&mut self, is_leader: bool) -> Result<(), Box<dyn Error>> {
        self.protocol_mut().set_leader(is_leader);
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

    pub fn malicious_notify(&self) -> Arc<Notify> {
        self.malicious_trigger_notify.clone()
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

    pub fn other_consensus_node(&self) -> Vec<PeerId> {
        self.other_consensus_node.clone()
    }

    pub fn divide_consensus_nodes(&self, ambiguous_count: u16) -> (Vec<PeerId>, Vec<PeerId>) {
        let nodes = self.other_consensus_node();
        if ambiguous_count as usize <= self.other_consensus_node().len() {
            (
                nodes[ambiguous_count as usize..].to_vec(),
                nodes[..ambiguous_count as usize].to_vec(),
            )
        } else {
            (vec![], nodes[..ambiguous_count as usize].to_vec())
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
                let d = match m {
                    MaliciousMode::LeaderFeignDeath(d, _)
                    | MaliciousMode::LeaderSendAmbiguousMessage(d, _, _, _)
                    | MaliciousMode::LeaderDelaySendMessage(d, _)
                    | MaliciousMode::LeaderSendDuplicateMessages(d, _)
                    | MaliciousMode::ReplicaNodeConspireForgeMessages(d, _, _, _) => d,
                };

                self.malicious_timer_start(d);
            }
            ConsensusNodeMode::Crash(internal) => {
                println!("I'm a crash consensus node in future!");
                self.crash_timer_start(internal);
            }
        }
    }

    pub fn reset(&mut self, mode: ConsensusNodeMode) {
        self.request_buffer.clear();
        self.malicious_flag = false;

        // let topic = IdentTopic::new("Consensus");
        // if let Err(e) = self
        //     .peer_mut()
        //     .network_swarm_mut()
        //     .behaviour_mut()
        //     .gossipsub
        //     .unsubscribe(&topic)
        // {
        //     eprintln!("Unsubscribe consensus topic error:{:?}", e);
        // };

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

    pub fn malicious_timer_start(&mut self, duration: u64) {
        println!("\nMalicious timer start...");
        let duration = Duration::from_millis(duration);
        let notify = self.malicious_trigger_notify.clone();

        tokio::spawn(async move {
            let start = Instant::now() + duration;
            let mut intv = interval_at(start, duration);

            intv.tick().await;
            notify.notify_one();
        });
    }

    // The consensus's handler on the controller message
    pub async fn running_controller_message_handler(&mut self, message: Message) {
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
            InteractiveMessage::MakeAConsensusRequest(request) => {
                self.write_requests_into_buffer(vec![request]);

                println!("^^^^^^^^^^^Current Request Buffer^^^^^^^^^^^");
                println!("{:?}", &self.request_buffer);

                if 0 != self.request_buffer.len() && self.protocol_mut().is_leader() {
                    *self.consensus_state.lock().await = true;
                }
            }
            InteractiveMessage::MakeConsensusRequests(requests) => {
                self.write_requests_into_buffer(requests);
                println!("^^^^^^^^^^^Current Request Buffer^^^^^^^^^^^");
                println!("{:?}", &self.request_buffer);
                if 0 != self.request_buffer.len() && self.protocol_mut().is_leader() {
                    *self.consensus_state.lock().await = true;
                }
            }
            _ => {}
        };
    }

    pub async fn no_running_controller_message_handler(&mut self, message: Message) {
        match message.interactive_message {
            // InteractiveMessage::ComponentInfo(Component::Controller(id_bytes)) => {
            //     let controller_id = PeerId::from_bytes(&id_bytes[..]).unwrap();
            //     println!("Controller PeerId: {:?}", controller_id.to_string());
            //     self.add_controller(controller_id);
            // }
            // InteractiveMessage::ComponentInfo(Component::Analyzer(id_bytes)) => {
            //     let controller_id = PeerId::from_bytes(&id_bytes[..]).unwrap();
            //     println!("Analyzer PeerId: {:?}", controller_id.to_string());
            //     self.add_analyzer(controller_id);
            // }
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
            InteractiveMessage::MaliciousTestPreparation => {
                let phases_map = self.protocol_mut().protocol_phases();

                phases_map.iter().for_each(|(k, v)| {
                    let message = Message {
                        interactive_message: InteractiveMessage::ConsensusPhase(
                            k.clone(),
                            v.clone(),
                        ),
                        source: vec![],
                    };
                    let serialized_message = coder::serialize_into_bytes(&message);
                    let send_type = SendType::Unicast(self.controller_id(), serialized_message);

                    self.send_message(send_type);
                });
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
                        if self.controller_id().to_string().eq(&peer_id.to_string()) {
                            let message: Message = coder::deserialize_for_bytes(&message.data);
                            self.no_running_controller_message_handler(message).await;
                        }
                    }
                    SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                        propagation_source: _peer_id,
                        message_id: _id,
                        message,
                    })) => {
                        if !self.verify_initialization() {
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
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                        for (peer, _) in list {
                            println!("Discovered {:?}", &peer);
                            self.peer_mut().network_swarm_mut().behaviour_mut().unicast.add_node_to_partial_view(&peer);
                            self.peer_mut().network_swarm_mut().behaviour_mut().gossipsub.add_explicit_peer(&peer);
                            self.other_consensus_node.push(peer);
                        }
                        //println!("Connected_nodes: {:?}", self.connected_nodes.lock().await);
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Expired(list))) => {
                        let swarm = self.peer_mut().network_swarm_mut();
                        for (peer, _) in list {
                            if !swarm.behaviour_mut().mdns.has_node(&peer) {
                                swarm.behaviour_mut().unicast.remove_node_from_partial_view(&peer);
                                swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer);
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

    pub fn generate_a_error_message(&mut self, msg: &[u8], field: Vec<String>) -> Vec<u8> {
        let parsed: Value = serde_json::from_slice(msg).unwrap();
        let map: Map<String, Value> = parsed.as_object().unwrap().clone();

        println!("Parse map: {:?}", map);

        let map_clone = map.clone();

        let motified_map = motify_map_value_with_field(map_clone, field);

        let map_json_str = serialize_into_json_str(&json!(motified_map));
        println!("After motify json: {:?}", map_json_str);

        map_json_str.into_bytes()
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
            }
            SendType::Unicast(receiver, msg) => {
                self.peer_mut()
                    .network_swarm_mut()
                    .behaviour_mut()
                    .unicast
                    .send_message(&receiver, msg);
            }
            SendType::AmbiguousBroadcast(msg, amsg, amsg_count) => {
                let (nodes, ambiguous_nodes) = self.divide_consensus_nodes(amsg_count);
                nodes.iter().for_each(|p| {
                    self.peer_mut()
                        .network_swarm_mut()
                        .behaviour_mut()
                        .unicast
                        .send_message(p, msg.clone());
                });
                ambiguous_nodes.iter().for_each(|p| {
                    self.peer_mut()
                        .network_swarm_mut()
                        .behaviour_mut()
                        .unicast
                        .send_message(p, amsg.clone());
                });
            }
        }
    }

    pub fn malicious_trigger(&mut self, phase: u8, send_type: SendType) {
        let current_request = self.protocol_mut().current_request();
        match self.mode() {
            ConsensusNodeMode::Dishonest(MaliciousMode::LeaderFeignDeath(_, p)) if p == phase => {}
            ConsensusNodeMode::Dishonest(MaliciousMode::LeaderSendDuplicateMessages(_, p))
                if p == phase =>
            {
                self.send_message(send_type.clone());
                self.send_message(send_type);
            }
            ConsensusNodeMode::Dishonest(MaliciousMode::LeaderSendAmbiguousMessage(
                _,
                p,
                field,
                amsg_count,
            )) if p == phase => {
                let msg = match send_type {
                    SendType::Broadcast(ref msg) => msg,
                    SendType::Unicast(_, ref msg) => msg,
                    SendType::AmbiguousBroadcast(ref msg, _, _) => msg,
                };
                let amsg = self.generate_a_error_message(&msg, field);
                let new_send_type = SendType::AmbiguousBroadcast(msg.clone(), amsg, amsg_count);
                self.send_message(new_send_type);
            }
            ConsensusNodeMode::Dishonest(MaliciousMode::LeaderDelaySendMessage(_, p))
                if p == phase => {}
            ConsensusNodeMode::Dishonest(MaliciousMode::ReplicaNodeConspireForgeMessages(
                _,
                p,
                field,
                request,
            )) if (p == phase && request == current_request) => {
                let new_send_type = match send_type {
                    SendType::Broadcast(ref msg) => {
                        let amsg = self.generate_a_error_message(&msg, field);
                        SendType::Broadcast(amsg)
                    }
                    SendType::Unicast(peer_id, ref msg) => {
                        let amsg = self.generate_a_error_message(&msg, field);
                        SendType::Unicast(peer_id, amsg)
                    }
                    _ => send_type,
                };
                self.send_message(new_send_type);
            }
            _ => {
                self.send_message(send_type);
            }
        }
    }

    pub async fn check_protocol_phase_state(&mut self, phase_state: PhaseState) {
        match phase_state {
            PhaseState::Over(request) => {
                let consensus_end_data = ConsensusEndData {
                    request: request.clone(),
                    completed_time: Local::now().timestamp_millis() as u64,
                };
                let data = ConsensusData::ConsensusEndData(consensus_end_data);
                let consensus_data_message = ConsensusDataMessage { data };

                let serialized_consensus_data_message =
                    coder::serialize_into_json_bytes(&consensus_data_message);

                let analysis_node_id = self.analyzer_id();
                self.peer_mut()
                    .network_swarm_mut()
                    .behaviour_mut()
                    .unicast
                    .send_message(&analysis_node_id, serialized_consensus_data_message);

                let request_hash = get_request_hash(&request);
                self.request_buffer.remove(&request_hash);
                if 0 != self.request_buffer.len() && self.protocol_mut().is_leader() {
                    *self.consensus_state.lock().await = true;
                }
            }
            PhaseState::ContinueExecute(send_queue) => {
                let mut send_queue = send_queue.clone();
                loop {
                    let send_type = send_queue.pop_front();
                    println!("SendType:{:?}", send_type);
                    if let Some(send_type) = send_type {
                        match send_type {
                            SendType::Broadcast(msg) => {
                                let phase = self.protocol_mut().get_current_phase(&msg[..]);
                                let send_type = SendType::Broadcast(msg);
                                if self.malicious_flag {
                                    self.malicious_trigger(phase, send_type.clone());
                                } else {
                                    self.send_message(send_type);
                                }
                            }
                            SendType::Unicast(r, msg) => {
                                let phase = self.protocol_mut().get_current_phase(&msg[..]);
                                let send_type = SendType::Unicast(r, msg);

                                if self.malicious_flag && self.is_leader {
                                    self.malicious_trigger(phase, send_type.clone());
                                } else {
                                    self.send_message(send_type);
                                }
                            }
                            _ => {}
                        }
                    } else {
                        break;
                    }
                }
            }
        }
    }

    pub fn state_check_start(&mut self) {
        println!("==============【Proposal state check】==============");
        // let is_primary = self.state.is_primary.clone();
        // let have_request = self.state.hava_request.clone();
        let state = self.consensus_state.clone();
        let consensus_notify = self.consensus_notify.clone();
        tokio::spawn(async move {
            loop {
                //println!("is_primary:{} ----- have_request:{}", *is_primary.lock().await, *have_request.lock().await);
                let state_value = *state.lock().await;
                if state_value {
                    consensus_notify.notify_one();
                    *state.lock().await = false;
                }
            }
        });
    }

    pub async fn message_handler_start(&mut self) {
        let consensus_notify = self.consensus_notify();
        let view_timeout_notify = self.view_timeout_notify();
        let protocol_stop_notify = self.protocol_stop_notify();
        let crash_notify = self.crash_notify();
        let malicious_notify = self.malicious_notify();

        self.protocol_start_preprocess();
        self.state_check_start();
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
                _ = malicious_notify.notified() => {
                    self.malicious_flag = true;
                }
                // start consensus notify
                _ = consensus_notify.notified() => {
                    // get a request in buffer
                    let request = self.next_request();
                    if let None = request {
                        continue;
                    }

                    let request = request.unwrap();
                    let serialized_request_message = self.protocol_mut().generate_serialized_request_message(&request);

                    let consensus_start_data = ConsensusStartData { request: request.clone(), start_time: Local::now().timestamp_millis() as u64 };
                    let consensus_data_message = ConsensusDataMessage {
                        data: ConsensusData::ConsensusStartData(consensus_start_data),
                    };
                    let serialized_consensus_data = coder::serialize_into_json_bytes(&consensus_data_message);
                    let analyzer_id = self.analyzer_id();
                    self.peer_mut().network_swarm_mut().behaviour_mut().unicast.send_message(&analyzer_id, serialized_consensus_data);

                    self.protocol_mut().set_current_request(&request);
                    // handle request
                    let phase_state = self.protocol_mut().consensus_protocol_message_handler(&serialized_request_message);
                    self.check_protocol_phase_state(phase_state).await;
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
                            self.running_controller_message_handler(message).await;
                        } else {
                            let phase_state = self.protocol_mut().consensus_protocol_message_handler(&message.data);
                            self.check_protocol_phase_state(phase_state).await;
                        };
                    }
                    SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                        propagation_source: _peer_id,
                        message_id: _id,
                        message,
                    })) => {
                        let phase_state = self.protocol_mut().consensus_protocol_message_handler(&message.data);
                        self.check_protocol_phase_state(phase_state).await;
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
pub type NumberOfDishonest = u16;
pub type AmbiguousField = Vec<String>;

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
    LeaderSendAmbiguousMessage(
        TriggerTimeInterval,
        ProtocolPhase,
        AmbiguousField,
        NumberOfAmbiguousMessage,
    ),
    LeaderDelaySendMessage(TriggerTimeInterval, ProtocolPhase),
    LeaderSendDuplicateMessages(TriggerTimeInterval, ProtocolPhase),

    ReplicaNodeConspireForgeMessages(TriggerTimeInterval, ProtocolPhase, AmbiguousField, Request),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ConfigureState {
    First,
    Other,
}
