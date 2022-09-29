use std::{collections::HashMap, error::Error, sync::Arc, time::Duration};

use cli::cmd::rootcmd::CMD;
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

use chrono::Local;
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    sync::Notify,
    time::{interval_at, Instant},
};
use utils::coder::{self};

use crate::message::{
    Component, ConsensusData, ConsensusDataMessage, InteractiveMessage, Message, TestItem,
};

use clap::{ArgMatches, Command as clap_Command};

use super::data_warehouse::DataWarehouse;

// The analyzer is responsible for the data acquisition and analysis of the protocol operation
pub struct Analyzer {
    // analyzer node's id
    id: PeerId,
    // p2p network communication
    peer: Peer,
    // hashmap of currently connected nodes
    connected_nodes: HashMap<String, PeerId>,

    // controller node's id
    controller_id: PeerId,
    // list of other analyzer nodes
    other_analyzer_node: Vec<PeerId>,

    // the test item currently being executed
    current_test_item: Option<TestItem>,
    performance_test_duration: u64,
    performance_test_internal: u64,
    crash_test_duration: u64,
    malicious_test_duration: u64,
    // a data warehouse where protocol running data is stored and analyzed
    data_warehouse: DataWarehouse,

    completed_test_notify: Arc<Notify>,
    internal_notify: Arc<Notify>,

    // client parameter channel,
    args_sender: Sender<Vec<String>>,
    args_recevier: Receiver<Vec<String>>,
}

impl Analyzer {
    pub fn new(
        peer: Peer,
        performance_test_duration: u64,
        performance_test_internal: u64,
        crash_test_duration: u64,
        malicious_test_duration: u64,
    ) -> Self {
        // let db_path = format!("./storage/data/{}_public_keys", port);
        let (args_sender, args_recevier) = mpsc::channel::<Vec<String>>(10);

        // let matches = ANALYZER_CMD.clone().get_matches();
        Self {
            id: peer.id,
            peer,
            connected_nodes: HashMap::new(),
            controller_id: PeerId::random(),
            other_analyzer_node: Vec::new(),
            current_test_item: None,
            performance_test_duration,
            performance_test_internal,
            crash_test_duration,
            malicious_test_duration,
            data_warehouse: DataWarehouse::new("mysql://root:root@localhost:3306/bft_diagnosis"),
            completed_test_notify: Arc::new(Notify::new()),
            internal_notify: Arc::new(Notify::new()),

            // client: Client::new(matches),
            args_sender,
            args_recevier,
        }
    }

    // Analyzer network startup, including peer start, gossip topic subscription, message handler
    pub async fn peer_start(&mut self) -> Result<(), Box<dyn Error>> {
        self.data_warehouse_mut().create_result_tables();
        self.peer_mut().swarm_start(false).await?;

        // let arg_sender = self.args_sender.clone();
        // self.client().run(arg_sender, 2);
        self.subscribe_topics();
        self.message_handler_start().await;

        Ok(())
    }

    // subscribe gossip topics
    pub fn subscribe_topics(&mut self) {
        let topic = IdentTopic::new("Initialization");
        if let Err(e) = self
            .peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic)
        {
            eprintln!("Subscribe error:{:?}", e);
        };
    }

    // get analyzer peer id
    pub fn id(&self) -> PeerId {
        self.id.clone()
    }

    // get analyzer peer id bytes
    pub fn id_bytes(&self) -> Vec<u8> {
        self.id.to_bytes()
    }

    // get mutable peer
    pub fn peer_mut(&mut self) -> &mut Peer {
        &mut self.peer
    }

    // get controller peer id
    pub fn controller_id(&self) -> PeerId {
        self.controller_id.clone()
    }

    // get client arguments sender
    pub fn args_sender(&self) -> Sender<Vec<String>> {
        self.args_sender.clone()
    }

    // add controller id
    pub fn add_controller(&mut self, controller_id: PeerId) {
        self.controller_id = controller_id;
    }

    // set the test item
    pub fn set_test_item(&mut self, test_item: TestItem) {
        self.current_test_item = Some(test_item);
    }

    pub fn clear_test_item(&mut self) {
        self.current_test_item = None;
    }

    // get current test item
    pub fn current_test_item(&self) -> Option<TestItem> {
        self.current_test_item.clone()
    }

    // get mutable data warehouse
    pub fn data_warehouse_mut(&mut self) -> &mut DataWarehouse {
        &mut self.data_warehouse
    }

    // get completed test notify
    pub fn completed_test_notify(&self) -> Arc<Notify> {
        self.completed_test_notify.clone()
    }

    // get internal test notify
    pub fn internal_notify(&self) -> Arc<Notify> {
        self.internal_notify.clone()
    }

    pub async fn performance_test_timer_start(&self) {
        println!("\nPerformance test timer start...");
        println!(
            "{}, {}",
            &self.performance_test_duration, &self.performance_test_internal
        );
        let duration = Duration::from_millis(self.performance_test_duration);
        let internal = Duration::from_millis(self.performance_test_internal);
        // println!("Current view timeout: {}", self.state.current_view_timeout);
        let internal_notify = self.internal_notify();
        let completed_test_notify = self.completed_test_notify();
        let timer_stop = Arc::new(Notify::new());
        let timer_stop_clone = timer_stop.clone();

        tokio::spawn(async move {
            let start = Instant::now();
            let mut intv = interval_at(start, internal);

            loop {
                tokio::select! {
                    _ = intv.tick() => {
                        internal_notify.notify_one();
                    }
                    _ = timer_stop.notified() => {
                        break;
                    }
                }
            }
        });

        tokio::spawn(async move {
            let start = Instant::now() + duration;
            let mut intv = interval_at(start, duration);

            intv.tick().await;
            completed_test_notify.notify_one();
            timer_stop_clone.notify_one();
        });
    }

    pub async fn crash_test_timer_start(&self) {
        println!("\nCrash test timer start...");
        println!("{}", &self.crash_test_duration);
        let duration = Duration::from_millis(self.crash_test_duration);
        // println!("Current view timeout: {}", self.state.current_view_timeout);
        let completed_test_notify = self.completed_test_notify();

        tokio::spawn(async move {
            let start = Instant::now() + duration;
            let mut intv = interval_at(start, duration);

            intv.tick().await;
            completed_test_notify.notify_one();
        });
    }

    pub async fn malicious_test_timer_start(&self) {
        println!("\nMalicious test timer start...");
        println!("{}", &self.malicious_test_duration);
        let duration = Duration::from_millis(self.malicious_test_duration);
        let completed_test_notify = self.completed_test_notify();

        tokio::spawn(async move {
            let start = Instant::now() + duration;
            let mut intv = interval_at(start, duration);

            intv.tick().await;
            completed_test_notify.notify_one();
        });
    }

    // analyzer initialize function
    pub fn init(&mut self) {
        let id_bytes = self.id_bytes();
        let component = Component::Analyzer(id_bytes);
        let interactive_message = InteractiveMessage::ComponentInfo(component);
        let message = Message {
            interactive_message,
            source: self.id_bytes().clone(),
        };

        let serialized_message = coder::serialize_into_bytes(&message);
        // let connected_nodes = self.connected_nodes.clone();
        // connected_nodes.iter().for_each(|(_, v)| {
        //     self.peer_mut()
        //         .network_swarm_mut()
        //         .behaviour_mut()
        //         .unicast
        //         .send_message(v, serialized_message.clone());
        // });

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

    // When a test start command is received from the controller,
    // the analyzer acts on the current test item
    pub fn compute_and_analyse(&mut self) {
        if let Some(ref test_item) = self.current_test_item() {
            let data_warehouse = self.data_warehouse_mut();
            match test_item {
                TestItem::Throughput => {
                    data_warehouse.compute_throughput();
                }
                TestItem::Latency => {
                    data_warehouse.compute_latency();
                }
                TestItem::ThroughputAndLatency => {
                    data_warehouse.compute_throughput();
                    data_warehouse.compute_latency();
                }
                TestItem::Scalability(count, _max, _) => {
                    data_warehouse.compute_scalability(*count, test_item);
                }
                TestItem::Crash(count, _) => {
                    data_warehouse.test_crash(*count);
                }
                TestItem::Malicious(m) => {
                    data_warehouse.test_malicious(&test_item);
                    println!("MaliciousBehaviour Test: {:?}", m);
                }
            }
        } else {
            println!("The current test item is empty. Please configure the test item!");
        };
    }

    // When the analyzer determines that the current round of testing has been completed,
    // it notifies the controller to proceed with the next test
    pub fn next_test(&mut self) {
        let current_test_item = self.current_test_item().unwrap();

        self.clear_test_item();
        let message = Message {
            interactive_message: InteractiveMessage::CompletedTest(current_test_item),
            source: self.id_bytes().clone(),
        };
        let serialized_message = coder::serialize_into_bytes(&message);
        let controller_id = self.controller_id();
        self.peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .unicast
            .send_message(&controller_id, serialized_message);
    }

    // A function that processes messages from the controller
    pub async fn controller_message_handler(&mut self, message: Message) {
        match message.interactive_message {
            InteractiveMessage::ComponentInfo(Component::Controller(id_bytes)) => {
                let controller_id = PeerId::from_bytes(&id_bytes[..]).unwrap();
                println!("Controller PeerId: {:?}", controller_id.to_string());
                self.add_controller(controller_id);
                // self.connected_nodes.remove(&controller_id.to_string());
            }
            InteractiveMessage::ComponentInfo(Component::Analyzer(_id_bytes)) => {}
            InteractiveMessage::TestItem(item) => {
                println!("#############################################");
                println!("Configure test item: {:?}", &item);
                self.set_test_item(item.clone());
                match item {
                    TestItem::Scalability(_, _, _) => {
                        self.data_warehouse_mut().insert_scalability_item(&item)
                    }
                    _ => {}
                }
                self.data_warehouse_mut().reset();
            }
            InteractiveMessage::CrashNode(count, peer_id_bytes) => {
                if let Some(peer_id_bytes) = peer_id_bytes {
                    let peer_id = PeerId::from_bytes(&peer_id_bytes[..]).unwrap();
                    self.data_warehouse_mut().record_crash_node(count, peer_id);
                }
            }
            InteractiveMessage::StartTest(_) => {
                let internal = self.performance_test_internal;
                self.data_warehouse_mut().set_throughput_internal(internal);
                println!("StartTest");
                if let Some(test_item) = self.current_test_item() {
                    match test_item {
                        TestItem::Throughput
                        | TestItem::Latency
                        | TestItem::ThroughputAndLatency
                        | TestItem::Scalability(_, _, _) => {
                            self.performance_test_timer_start().await;
                        }
                        TestItem::Crash(_, _) => {
                            self.crash_test_timer_start().await;
                        }
                        TestItem::Malicious(_) => {
                            self.malicious_test_timer_start().await;
                        }
                    }
                    // self.compute_and_analyse();
                }
            }
            _ => {}
        };
    }

    // A function that processes messages from the consensus node
    pub fn consensus_node_message_handler(
        &mut self,
        origin_peer_id: &PeerId,
        message: ConsensusDataMessage,
    ) {
        match message.data {
            ConsensusData::ConsensusStartData(data) => {
                println!(
                    "【ConsensusStartData(from {:?})】: {:?}",
                    origin_peer_id.to_string(),
                    &data
                );
                self.data_warehouse
                    .store_consensus_start_data(*origin_peer_id, data);
            }
            ConsensusData::ConsensusEndData(data) => {
                println!(
                    "【ConsensusEndData(from {:?})】: {:?}",
                    origin_peer_id.to_string(),
                    &data
                );
                self.data_warehouse
                    .store_consensus_end_data(*origin_peer_id, data);
            }
        };
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
            println!("\nAnalyzer Initialization！");
            self.init();
        }

        if let Some(_) = matches.subcommand_matches("printThroughputResults") {
            self.data_warehouse_mut().print_throughput_results();
        }

        if let Some(_) = matches.subcommand_matches("printLatencyResults") {
            self.data_warehouse_mut().print_latency_results();
        }

        if let Some(_) = matches.subcommand_matches("printScalabilityThroughputResults") {
            self.data_warehouse_mut()
                .print_scalability_throughput_results();
        }

        if let Some(_) = matches.subcommand_matches("printScalabilityLatencyResults") {
            self.data_warehouse_mut()
                .print_scalability_latency_results();
        }

        if let Some(_) = matches.subcommand_matches("printCrashResults") {
            self.data_warehouse_mut().print_crash_results();
        }

        if let Some(_) = matches.subcommand_matches("printMaliciousResults") {
            self.data_warehouse_mut().print_malicious_results();
        }

        // if let Some(ref matches) = matches.subcommand_matches("test") {
        //     if let Some(_) = matches.subcommand_matches("pbft") {
        //         println!("开始进行PBFT共识协议的测试");
        //     };

        //     if let Some(_) = matches.subcommand_matches("hotstuff") {
        //         let rt = tokio::runtime::Runtime::new().unwrap();
        //         let async_req = async { println!("开始进行hotstuff共识协议的测试") };
        //         rt.block_on(async_req);
        //     };

        //     if let Some(_) = matches.subcommand_matches("chain_hotstuff") {
        //         let rt = tokio::runtime::Runtime::new().unwrap();
        //         let async_req =
        //             async { println!("开始进行chain_hotstuff共识协议的测试") };
        //         rt.block_on(async_req);
        //     };
        // }
    }

    // Function launched by the Message handler
    pub async fn message_handler_start(&mut self) {
        // let mut stdin = io::BufReader::new(io::stdin()).lines();

        let completed_test_notify = self.completed_test_notify();
        let internal_notify = self.internal_notify();

        loop {
            tokio::select! {
                Some(args) = self.args_recevier.recv() => {
                    self.before_cmd_match(args);
                },
                _ = internal_notify.notified() => {
                    println!("Internal");
                    self.compute_and_analyse();
                }
                _ = completed_test_notify.notified() => {
                    self.compute_and_analyse();
                    println!("Completed");
                    self.next_test();
                },
                event = self.peer.network_swarm_mut().select_next_some() => match event {
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        let peer_id = PeerId::from_bytes(&message.source[..]).unwrap();
                        if self.controller_id().to_string().eq(&peer_id.to_string()) {
                            println!("");
                            let message: Message = coder::deserialize_for_bytes(&message.data);
                            self.controller_message_handler(message).await;
                        } else if self.connected_nodes.contains_key(&peer_id.to_string()) {
                            let consensus_data_message: ConsensusDataMessage = coder::deserialize_for_json_bytes(&message.data);
                            self.consensus_node_message_handler(&peer_id, consensus_data_message);
                        }
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
                                println!("\nController is: {:?}", &controller_id);
                                self.add_controller(controller_id);
                            },
                            InteractiveMessage::ComponentInfo(Component::Analyzer(_id_bytes)) => {

                            },
                            _ => {}
                        };

                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                        // let swarm = self.peer_mut().network_swarm_mut();
                        for (peer, _) in list {
                            println!("Discovered {:?}", &peer);
                            self.peer_mut().network_swarm_mut().behaviour_mut().unicast.add_node_to_partial_view(&peer);
                            self.peer_mut().network_swarm_mut().behaviour_mut().gossipsub.add_explicit_peer(&peer);
                            self.connected_nodes.insert(peer.to_string(), peer.clone());
                        }
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
                        println!("\nListening on {:?}", address);
                    }
                    _ => {}
                }
            }
        }
    }
}
