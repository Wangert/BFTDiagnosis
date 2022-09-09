use std::{
    collections::{HashMap, HashSet},
    error::Error,
    sync::Arc,
    thread::sleep,
    time::Duration,
};

use chrono::Local;
use cli::{
    client::Client,
    cmd::rootcmd::{CMD, CONTROLLER_CMD},
};
use libp2p::{
    futures::StreamExt,
    gossipsub::{GossipsubEvent, IdentTopic},
    mdns::MdnsEvent,
    swarm::SwarmEvent,
    PeerId,
};
use network::{
    p2p_protocols::{
        base_behaviour::{BaseBehaviour, OutEvent},
        unicast::behaviour::UnicastEvent,
    },
    peer::Peer,
};

use tokio::{
    io::{self, AsyncBufReadExt},
    sync::{
        mpsc::{self, Receiver, Sender},
        Notify,
    },
};
use utils::coder::{self, serialize_into_bytes};

use crate::{
    basic_consensus_node::ConsensusNodeMode,
    common::{generate_bls_keys, generate_consensus_requests_command},
    message::{
        Command, CommandMessage, Component, ConsensusNodePKInfo, InteractiveMessage,
        MaliciousAction, Message, TestItem,
    },
};

use clap::{ArgMatches, Command as clap_Command};

use super::{config::BFTDiagnosisConfig, executor::Executor};

pub struct Controller {
    id: PeerId,
    peer: Peer,
    executor: Executor,
    connected_nodes: HashMap<String, PeerId>,

    consensus_nodes: HashSet<PeerId>,
    waiting_consensus_nodes: HashSet<PeerId>,

    consensus_node_mode_state: HashMap<PeerId, bool>,

    not_crash_consensus_nodes: HashSet<PeerId>,
    crash_consensus_nodes: HashSet<PeerId>,
    optional_crash_consensus_nodes: HashSet<PeerId>,

    analyzer_id: PeerId,
    test_items: Vec<TestItem>,

    next_test_item: Option<TestItem>,
    next_mid_test_item: Option<TestItem>,

    next_test_flag: bool,
    feedback_crash_node: PeerId,

    reset_success_count: u16,
    crash_reset_success_count: u16,

    client: Client,
    args_sender: Sender<Vec<String>>,
    args_recevier: Receiver<Vec<String>>,
}

impl Controller {
    pub fn new(peer: Peer, port: &str) -> Self {
        let db_path = format!("./storage/data/{}_public_keys", port);
        let executor = Executor::new(&db_path);
        let (args_sender, args_recevier) = mpsc::channel::<Vec<String>>(10);

        let matches = CONTROLLER_CMD.clone().get_matches();
        Self {
            id: peer.id,
            peer,
            executor,
            connected_nodes: HashMap::new(),

            consensus_nodes: HashSet::new(),
            waiting_consensus_nodes: HashSet::new(),

            consensus_node_mode_state: HashMap::new(),

            not_crash_consensus_nodes: HashSet::new(),
            crash_consensus_nodes: HashSet::new(),
            optional_crash_consensus_nodes: HashSet::new(),

            analyzer_id: PeerId::random(),
            test_items: Vec::new(),

            next_test_item: None,
            next_mid_test_item: None,

            next_test_flag: true,
            feedback_crash_node: PeerId::random(),

            reset_success_count: 0,
            crash_reset_success_count: 0,

            client: Client::new(matches),
            args_sender,
            args_recevier,
        }
    }

    pub fn configure(&mut self, bft_diagnosis_config: BFTDiagnosisConfig) {
        let mut throughput_flag = false;
        let mut latency_flag = false;

        if let Some(throughput) = bft_diagnosis_config.throughput {
            if let Some(true) = throughput.enable {
                throughput_flag = true;
            }
        }

        if let Some(latency) = bft_diagnosis_config.latency {
            if let Some(true) = latency.enable {
                latency_flag = true;
            }
        }

        if throughput_flag && latency_flag {
            self.add_test_item(TestItem::ThroughputAndLatency);
        } else if throughput_flag && !latency_flag {
            self.add_test_item(TestItem::Throughput);
        } else if !throughput_flag && latency_flag {
            self.add_test_item(TestItem::Latency);
        }

        if let Some(scalability) = bft_diagnosis_config.scalability {
            if matches!(scalability.enable, Some(true)) {
                let max = if let Some(max) = scalability.max {
                    max
                } else {
                    0
                };

                let internal = if let Some(internal) = scalability.internal {
                    internal
                } else {
                    0
                };

                _ = self.add_test_item(TestItem::Scalability(4, max, internal));
            }
        }

        if let Some(crash) = bft_diagnosis_config.crash {
            if matches!(crash.enable, Some(true)) {
                let max = if let Some(max) = crash.max { max } else { 0 };
                self.add_test_item(TestItem::Crash(0, max));
            }
        }

        if let Some(malicious) = bft_diagnosis_config.malicious {
            if matches!(malicious.enable, Some(true)) {
                if let Some(behaviour) = malicious.behaviour {
                    if matches!(
                        behaviour.cmp(&"action1".to_string()),
                        std::cmp::Ordering::Equal
                    ) {
                        self.add_test_item(TestItem::Malicious(MaliciousAction::Action1));
                    }
                }
            }
        }
    }

    pub async fn peer_start(&mut self) -> Result<(), Box<dyn Error>> {
        self.peer_mut().swarm_start(false).await?;
        let arg_sender = self.args_sender.clone();
        // self.client().run(arg_sender, 1);
        self.subscribe_topics();
        self.message_handler_start().await;

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

    // Controller's id
    pub fn id(&self) -> PeerId {
        self.id.clone()
    }

    // Controller's id bytes
    pub fn id_bytes(&self) -> Vec<u8> {
        self.id.to_bytes()
    }

    pub fn peer_mut(&mut self) -> &mut Peer {
        &mut self.peer
    }

    pub fn executor_mut(&mut self) -> &mut Executor {
        &mut self.executor
    }

    pub fn consensus_nodes_set(&mut self) -> &mut HashSet<PeerId> {
        &mut self.consensus_nodes
    }

    pub fn not_crash_consensus_nodes_set(&mut self) -> &mut HashSet<PeerId> {
        &mut self.not_crash_consensus_nodes
    }

    pub fn crash_consensus_nodes_set(&mut self) -> &mut HashSet<PeerId> {
        &mut self.crash_consensus_nodes
    }

    pub fn consensus_node_mode_state(&mut self) -> &mut HashMap<PeerId, bool> {
        &mut self.consensus_node_mode_state
    }

    pub fn optional_crash_consensus_nodes_set(&mut self) -> &mut HashSet<PeerId> {
        &mut self.optional_crash_consensus_nodes
    }

    pub fn waiting_consensus_nodes_set(&mut self) -> &mut HashSet<PeerId> {
        &mut self.waiting_consensus_nodes
    }

    pub fn remove_waiting_consensus_node(&mut self, peer_id: &PeerId) {
        println!("Remove!");
        println!("{:?}", &self.waiting_consensus_nodes);
        self.waiting_consensus_nodes.remove(peer_id);
        println!("{:?}", &self.waiting_consensus_nodes);
    }

    pub fn client(&self) -> Client {
        self.client.clone()
    }

    // Analyzer's id
    pub fn analyzer_id(&self) -> PeerId {
        self.analyzer_id.clone()
    }

    pub fn args_sender(&self) -> Sender<Vec<String>> {
        self.args_sender.clone()
    }

    pub fn add_analyzer(&mut self, analyzer_id: PeerId) {
        self.analyzer_id = analyzer_id;
    }

    // add test item
    pub fn add_test_item(&mut self, test_item: TestItem) -> usize {
        self.test_items.push(test_item);
        self.test_items.len()
    }

    // get next test item
    pub fn next_test_item(&mut self) -> Option<TestItem> {
        self.test_items.pop()
    }

    // Assign keys to consensus nodes and send all consensus node public keys to each consensus node
    pub fn assign_key_to_consensus_node(&mut self) {
        let distribute_tbls_key_vec = generate_bls_keys(&self.connected_nodes, 1);

        println!("key count: {}", distribute_tbls_key_vec.len());
        let key = distribute_tbls_key_vec[0].tbls_key.clone();
        let key_msg = CommandMessage {
            command: Command::AssignTBLSKeypair(key),
        };
        let serialized_key = coder::serialize_into_bytes(&key_msg);
        println!("{:?}", serialized_key);
        let de_key: CommandMessage = coder::deserialize_for_bytes(&serialized_key);
        println!("{:#?}", de_key);

        let mut consensus_node_pks: HashMap<Vec<u8>, ConsensusNodePKInfo> = HashMap::new();
        for key_info in distribute_tbls_key_vec {
            let msg = CommandMessage {
                command: Command::AssignTBLSKeypair(key_info.tbls_key.clone()),
            };
            let serialized_msg = coder::serialize_into_bytes(&msg);
            self.peer_mut()
                .network_swarm_mut()
                .behaviour_mut()
                .unicast
                .send_message(&key_info.peer_id, serialized_msg);

            let db_key = key_info.peer_id.clone().to_bytes();
            let consensus_node_pk_info = ConsensusNodePKInfo {
                number: key_info.number,
                public_key: key_info.tbls_key.public_key,
            };
            let db_value = coder::serialize_into_bytes(&consensus_node_pk_info);
            self.executor.db.write(&db_key, &db_value);
            consensus_node_pks.insert(db_key, consensus_node_pk_info);
        }

        let msg = CommandMessage {
            command: Command::DistributeConsensusNodePKsInfo(consensus_node_pks),
        };
        let serialized_msg = coder::serialize_into_bytes(&msg);
        let topic = IdentTopic::new("Consensus");
        if let Err(e) = self
            .peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), serialized_msg)
        {
            eprintln!("Publish message error:{:?}", e);
        }
    }

    // Initiate a set of consensus requests
    pub fn make_consensus_requests(&mut self, count: usize) {
        let msg = generate_consensus_requests_command(count);
        println!("Request: {:#?}", msg);
        let serialized_msg = serialize_into_bytes(&msg);
        let topic = IdentTopic::new("Consensus");
        if let Err(e) = self
            .peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), serialized_msg)
        {
            eprintln!("Publish message error:{:?}", e);
        }
    }

    // Controller Initialization
    pub fn init(&mut self) {
        let id_bytes = self.id_bytes();
        let component = Component::Controller(id_bytes);
        let interactive_message = InteractiveMessage::ComponentInfo(component);
        let message = Message {
            interactive_message,
            source: vec![],
        };

        let serialized_message = coder::serialize_into_bytes(&message);
        let topic = IdentTopic::new("Initialization");
        if let Err(e) = self
            .peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), serialized_message)
        {
            eprintln!("Publish message error:{:?}", e);
        }
    }

    // The controller configures the analyzer parameters
    pub fn configure_analyzer(&mut self) -> bool {
        let test_item = self.next_test_item();
        if let Some(item) = test_item {
            match item {
                TestItem::Crash(_, _) => {
                    self.optional_crash_consensus_nodes =
                        self.waiting_consensus_nodes_set().clone();
                }
                _ => {}
            }
            let message = Message {
                interactive_message: InteractiveMessage::TestItem(item),
                source: vec![],
            };
            let serialized_message = coder::serialize_into_bytes(&message);

            let analyzer_id = self.analyzer_id();
            self.peer_mut()
                .network_swarm_mut()
                .behaviour_mut()
                .unicast
                .send_message(&analyzer_id, serialized_message);

            true
        } else {
            println!("Protocol Test Completed!!!");
            false
        }
    }

    pub fn configure_consensus_node(&mut self) {
        let mut message = Message {
            interactive_message: InteractiveMessage::SubscribeConsensusTopic(
                Local::now().timestamp_millis() as u64,
            ),
            source: self.id_bytes().clone(),
        };

        let mut serialized_message = coder::serialize_into_bytes(&message);

        let waiting_consensus_nodes = self.waiting_consensus_nodes.clone();
        println!(
            "Current Waiting Consensus Nodes: {:?}",
            &waiting_consensus_nodes
        );
        for peer_id in waiting_consensus_nodes {
            self.peer_mut()
                .network_swarm_mut()
                .behaviour_mut()
                .unicast
                .send_message(&peer_id, serialized_message.clone());
        }
    }

    pub fn need_reset(&mut self) -> bool {
        self.consensus_nodes_set().len() as u16 != self.crash_reset_success_count
    }

    pub fn reset_consensus_node(&mut self) {
        if !self.need_reset() {
            let next_mid_test_item = self.next_mid_test_item.clone();
            if let Some(item) = next_mid_test_item {
                match item {
                    TestItem::Scalability(_, _, _) => {
                        self.next_mid_test(item, None);
                    }
                    TestItem::Crash(crash_count, _) => {
                        let crash_peer_id = self.random_crash_a_consensus_node();

                        if let Some(id) = crash_peer_id {
                            let imessage =
                                InteractiveMessage::CrashNode(crash_count, id.to_bytes());
                            self.next_mid_test(item.clone(), Some(imessage));
                        }
                    }
                    _ => {}
                }
            } else {
                self.next_test();
            }
        } else {
            // self.consensus_node_mode_state().clear();
            let message = Message {
                interactive_message: InteractiveMessage::Reset(
                    Local::now().timestamp_millis() as u64
                ),
                source: vec![],
            };

            let serialized_message = coder::serialize_into_bytes(&message);

            let consensus_nodes = self.consensus_nodes_set().clone();
            // println!(
            //     "[Reset_Consensus_node] Current Consensus Nodes: {:?}",
            //     &consensus_nodes
            // );
            for peer_id in consensus_nodes {
                self.peer_mut()
                    .network_swarm_mut()
                    .behaviour_mut()
                    .unicast
                    .send_message(&peer_id, serialized_message.clone());
            }
        }
    }

    pub fn next_test(&mut self) -> bool {
        self.configure_analyzer()
    }

    pub fn next_mid_test(&mut self, test_item: TestItem, option: Option<InteractiveMessage>) {
        let mut message = Message {
            interactive_message: InteractiveMessage::TestItem(test_item),
            source: self.id_bytes().clone(),
        };

        let mut serialized_message = coder::serialize_into_bytes(&message);

        let analyzer_id = self.analyzer_id();
        self.peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .unicast
            .send_message(&analyzer_id, serialized_message);

        if let Some(imessage) = option {
            message = Message {
                interactive_message: imessage,
                source: vec![],
            };

            serialized_message = coder::serialize_into_bytes(&message);
            self.peer_mut()
                .network_swarm_mut()
                .behaviour_mut()
                .unicast
                .send_message(&analyzer_id, serialized_message);
        }
    }

    pub fn print_unfinished_test_items(&self) {
        println!("\nCurrently unfinished test items：");
        for item in &self.test_items {
            match item {
                TestItem::Throughput => println!("Throughput"),
                TestItem::Latency => println!("Latency"),
                TestItem::ThroughputAndLatency => println!("ThroughputAndLatency"),
                TestItem::Scalability(_, _, _) => println!("Scalability"),
                TestItem::Crash(_, _) => println!("Crash"),
                TestItem::Malicious(action) => println!("Malicious({:?})", action),
            }
        }
    }

    pub fn set_consensus_node_mode(&mut self, peer_id: PeerId, mode: ConsensusNodeMode) {
        let message = Message {
            interactive_message: InteractiveMessage::ConsensusNodeMode(mode),
            source: self.id_bytes().clone(),
        };

        println!("{:?} set mode!!!!", peer_id);

        self.feedback_crash_node = peer_id.clone();
        let serialized_message = coder::serialize_into_bytes(&message);
        // for i in 1..4 {
        //     println!("{}", i);
        //     self.peer_mut()
        //     .network_swarm_mut()
        //     .behaviour_mut()
        //     .unicast
        //     .send_message(&peer_id, serialized_message.clone());
        // }

        self.peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .unicast
            .send_message(&peer_id, serialized_message.clone());
    }

    pub fn random_crash_a_consensus_node(&mut self) -> Option<PeerId> {
        if 0 == self.optional_crash_consensus_nodes_set().len() {
            eprintln!("No running consensus node");
            self.next_test();
            return None;
        } else {
            let peer_id = self
                .optional_crash_consensus_nodes_set()
                .iter()
                .last()
                .unwrap()
                .clone();
            let mode = ConsensusNodeMode::Crash(1000);
            self.set_consensus_node_mode(peer_id, mode);
            self.optional_crash_consensus_nodes_set().remove(&peer_id);

            return Some(peer_id);
        }
    }

    pub fn check_consensus_node_mode_feedback(&mut self) -> bool {
        self.consensus_nodes_set().len() == self.consensus_node_mode_state().len()
    }

    pub fn protocol_start(&mut self) {
        let message = Message {
            interactive_message: InteractiveMessage::ProtocolStart(
                Local::now().timestamp_millis() as u64
            ),
            source: self.id_bytes().clone(),
        };

        let serialized_message = coder::serialize_into_bytes(&message);

        let consensus_nodes = self.consensus_nodes_set().clone();
        // println!(
        //     "[Protocol_Start] Current Consensus Nodes: {:?}",
        //     &consensus_nodes
        // );
        for peer_id in consensus_nodes {
            self.peer_mut()
                .network_swarm_mut()
                .behaviour_mut()
                .unicast
                .send_message(&peer_id, serialized_message.clone());
        }
    }

    // Start a test
    pub fn start_test(&mut self) {
        let message = Message {
            interactive_message: InteractiveMessage::StartTest(
                Local::now().timestamp_millis() as u64
            ),
            source: self.id_bytes().clone(),
        };
        let serialized_message = coder::serialize_into_bytes(&message);
        let analyzer_id = self.analyzer_id();
        self.peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .unicast
            .send_message(&analyzer_id, serialized_message);
    }

    // The controller's handler on the analyzer message
    pub async fn analyzer_message_handler(&mut self, message: Message) {
        match message.interactive_message {
            InteractiveMessage::ComponentInfo(Component::Controller(id_bytes)) => {
                let controller_id = PeerId::from_bytes(&id_bytes[..]).unwrap();
                println!("Controller PeerId: {:?}", controller_id.to_string());
            }
            InteractiveMessage::ComponentInfo(Component::Analyzer(id_bytes)) => {
                let analyzer_id = PeerId::from_bytes(&id_bytes[..]).unwrap();
                println!("\nAnalyzer is: {:?}", analyzer_id.to_string());
                self.remove_waiting_consensus_node(&analyzer_id);
                self.add_analyzer(analyzer_id);
                println!(
                    "Current waiting consensus nodes: {:?}",
                    &self.waiting_consensus_nodes
                );
            }
            InteractiveMessage::CompletedTest(test_item) => {
                println!("Complete {:?} test.", test_item.clone());
                match test_item {
                    TestItem::Scalability(count, max, internal) => {
                        if count + internal <= max {
                            self.next_mid_test_item =
                                Some(TestItem::Scalability(count + internal, max, internal));
                            // self.next_mid_test(test_item, None);
                        } else {
                            self.next_mid_test_item = None;
                            // self.next_test_flag = self.next_test();
                        }
                    }
                    TestItem::Crash(count, max) => {
                        if count + 1 <= max {
                            self.next_mid_test_item = Some(TestItem::Crash(count + 1, max));

                            // let crash_peer_id = self.random_crash_a_consensus_node();

                            // if let Some(id) = crash_peer_id {
                            //     let imessage = InteractiveMessage::CrashNode(count+1, id.to_bytes());
                            //     self.next_mid_test(test_item, Some(imessage));
                            // }
                        } else {
                            self.next_mid_test_item = None;
                            // self.next_test_flag = self.next_test();
                        }
                    }
                    TestItem::Malicious(_) => {}
                    _ => {}
                }

                self.reset_consensus_node();
            }
            _ => {}
        };
    }

    // The controller's handler on the consensus node message
    pub fn consensus_node_message_handler(&mut self, peer_id: PeerId, message: Message) {
        match message.interactive_message {
            InteractiveMessage::SubscribeConsensusTopicSuccess => {
                println!("{:?} subscribe consensus topoc success!", &peer_id);
                self.remove_waiting_consensus_node(&peer_id);
                self.consensus_nodes_set().insert(peer_id);
                self.not_crash_consensus_nodes_set().insert(peer_id);
                self.optional_crash_consensus_nodes_set().insert(peer_id);
            }
            InteractiveMessage::ResetSuccess(mode) => match mode {
                ConsensusNodeMode::Uninitialized => todo!(),
                ConsensusNodeMode::Honest => {
                    self.reset_success_count += 1;
                    self.check_reset_all_success();
                }
                ConsensusNodeMode::Dishonest(_) => todo!(),
                ConsensusNodeMode::Crash(_) => {
                    self.crash_reset_success_count += 1;
                    self.crash_consensus_nodes_set().insert(peer_id);
                    self.not_crash_consensus_nodes_set().remove(&peer_id);
                    if !self.consensus_nodes_set().len() as u16 == self.crash_reset_success_count {
                        self.check_reset_all_success();
                    }
                }
            },
            InteractiveMessage::ConsensusNodeModeSuccess(mode) => {
                println!("Feedback:{:?}", &peer_id);
                match mode {
                    ConsensusNodeMode::Uninitialized => todo!(),
                    ConsensusNodeMode::Honest => {
                        println!("Honest consensus node: {:?}", &peer_id);
                    }
                    ConsensusNodeMode::Dishonest(_) => todo!(),
                    ConsensusNodeMode::Crash(_) => {
                        println!("Crash consensus node: {:?}", &peer_id);
                        if self
                            .feedback_crash_node
                            .to_string()
                            .eq(&peer_id.to_string())
                        {
                            // self.not_crash_consensus_nodes_set().remove(&peer_id);
                            println!("protocol start success!!!");
                            self.protocol_start();
                            self.start_test();
                        }
                    }
                }
            }
            _ => {}
        };
    }

    pub fn check_reset_all_success(&mut self) {
        println!("Consensus node count: {}", self.consensus_nodes_set().len());
        if self.consensus_nodes_set().len() as u16
            == (self.reset_success_count + self.crash_reset_success_count)
        {
            println!("All consensus node reset ok!!!");
            self.reset_success_count = 0;
            self.crash_reset_success_count = 0;
            let next_mid_test_item = self.next_mid_test_item.clone();
            if let Some(item) = next_mid_test_item {
                match item {
                    TestItem::Scalability(_, _, _) => {
                        self.next_mid_test(item, None);
                    }
                    TestItem::Crash(crash_count, _) => {
                        let crash_peer_id = self.random_crash_a_consensus_node();

                        if let Some(id) = crash_peer_id {
                            let imessage =
                                InteractiveMessage::CrashNode(crash_count, id.to_bytes());
                            self.next_mid_test(item.clone(), Some(imessage));
                        }
                    }
                    _ => {}
                }
            } else {
                self.next_test();
            }
        }
    }

    // Process client commands before matching
    pub fn before_cmd_match(&mut self, args: Vec<String>) {
        match clap_Command::try_get_matches_from(CMD.to_owned(), args.clone()) {
            Ok(matches) => {
                self.cmd_match(&matches);
            }
            Err(err) => {
                err.print().expect("Error writing Error");
            }
        };
    }

    // Match the client command
    pub fn cmd_match(&mut self, matches: &ArgMatches) {
        // let matches = self.client().arg_matches();

        if let Some(_) = matches.subcommand_matches("init") {
            println!("\nBFT测试平台初始化成功！");
            self.init();
        }

        if let Some(_) = matches.subcommand_matches("printUnfinishedTestItems") {
            // println!("\nBFT测试平台初始化成功！");
            self.print_unfinished_test_items();
        }

        if let Some(_) = matches.subcommand_matches("startTest") {
            // println!("\nBFT测试平台初始化成功！");
            self.start_test();
        }

        if let Some(_) = matches.subcommand_matches("protocolStart") {
            // println!("\nBFT测试平台初始化成功！");
            self.protocol_start();
        }

        if let Some(_) = matches.subcommand_matches("configureConsensusNode") {
            // println!("\nBFT测试平台初始化成功！");
            self.configure_consensus_node();
        }

        if let Some(_) = matches.subcommand_matches("configureAnalyzer") {
            // println!("\nBFT测试平台初始化成功！");
            self.configure_analyzer();
        }

    }

    // Framework network message handler startup function
    pub async fn message_handler_start(&mut self) {
        // let mut stdin = io::BufReader::new(io::stdin()).lines();

        loop {
            tokio::select! {
                Some(args) = self.args_recevier.recv() => {
                    // println!("{:?}", &args);
                    self.before_cmd_match(args);
                }

                event = self.peer.network_swarm_mut().select_next_some() => match event {
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        let peer_id = PeerId::from_bytes(&message.source[..]).unwrap();
                        if self.analyzer_id().to_string().eq(&peer_id.to_string()) {
                            let message: Message = coder::deserialize_for_bytes(&message.data);
                            self.analyzer_message_handler(message).await;

                        } else if self.waiting_consensus_nodes_set().contains(&peer_id) || self.consensus_nodes_set().contains(&peer_id) {
                            let message: Message = coder::deserialize_for_bytes(&message.data);
                            self.consensus_node_message_handler(peer_id, message);
                        }
                    }
                    SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                        propagation_source: _peer_id,
                        message_id: _id,
                        message,
                    })) => {
                        let message: Message = coder::deserialize_for_bytes(&message.data);
                        self.analyzer_message_handler(message).await;
                        //self.executor.message_handler(&self.id.to_bytes(), &message.data).await;
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                        // let swarm = self.peer.network_swarm_mut();
                        // let waiting_consensus_nodes = self.;
                        for (peer, _) in list {
                            println!("Discovered {:?}", &peer);
                            self.peer.network_swarm_mut().behaviour_mut().unicast.add_node_to_partial_view(&peer);
                            self.peer.network_swarm_mut().behaviour_mut().gossipsub.add_explicit_peer(&peer);
                            self.waiting_consensus_nodes_set().insert(peer);
                            // self.connected_nodes.insert(peer.to_string(), peer.clone());
                        }
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Expired(list))) => {
                        let swarm = self.peer.network_swarm_mut();
                        for (peer, _) in list {
                            if !swarm.behaviour_mut().mdns.has_node(&peer) {
                                swarm.behaviour_mut().unicast.remove_node_from_partial_view(&peer);
                                swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer);
                                // self.connected_nodes.remove(&peer.to_string());
                            }
                        }
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("\nListening on {:?}", address);
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
pub mod controller_test {
    use std::collections::HashSet;

    #[test]
    pub fn hashset_test() {
        let mut set = HashSet::new();
        set.insert(1);
        set.insert(2);
        set.insert(3);

        let last = set.iter().last().unwrap();
        println!("{:?}", set);
    }
}
