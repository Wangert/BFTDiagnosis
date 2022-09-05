use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

use cli::{client::Client, cmd::rootcmd::{CONTROLLER_CMD, CMD}};
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

use tokio::{io::{self, AsyncBufReadExt}, sync::mpsc::{Sender, Receiver, self}};
use utils::coder::{self, serialize_into_bytes};

use crate::{
    common::{generate_bls_keys, generate_consensus_requests_command},
    message::{
        Command, CommandMessage, Component, ConsensusNodePKInfo, InteractiveMessage,
        MaliciousAction, Message, TestItem,
    },
};

use clap::{Command as clap_Command, ArgMatches};

use super::{executor::Executor, config::BFTDiagnosisConfig};

pub struct Controller {
    id: PeerId,
    peer: Peer,
    executor: Executor,
    connected_nodes: HashMap<String, PeerId>,

    analyzer_id: PeerId,
    test_items: Vec<TestItem>,

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

            analyzer_id: PeerId::random(),
            test_items: Vec::new(),

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

                _ = self.add_test_item(TestItem::Scalability(max, internal));
            }
        }

        if let Some(crash) = bft_diagnosis_config.crash {
            if let Some(true) = crash.enable {
                self.add_test_item(TestItem::Crash(3));
            }
        }

        if let Some(malicious) = bft_diagnosis_config.malicious {
            if matches!(malicious.enable, Some(true)) {
                if let Some(behaviour) = malicious.behaviour {
                    if matches!(behaviour.cmp(&"action1".to_string()), std::cmp::Ordering::Equal) {
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

        self.message_handler_start().await;

        Ok(())
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
    pub fn configure_analyzer(&mut self) {
        let test_item = self.next_test_item();
        if let Some(item) = test_item {
            let message = Message {
                interactive_message: InteractiveMessage::TestItem(item),
            };
            let serialized_message = coder::serialize_into_bytes(&message);

            let analyzer_id = self.analyzer_id();
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
                TestItem::Scalability(_, _) => println!("Scalability"),
                TestItem::Crash(_) => println!("Crash"),
                TestItem::Malicious(action) => println!("Malicious({:?})", action),
            }
        }
    }

    // Start a test
    pub fn start_test(&mut self) {
        let message = Message {
            interactive_message: InteractiveMessage::StartTest,
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
    pub fn analyzer_message_handler(&mut self, message: Message) {
        match message.interactive_message {
            InteractiveMessage::ComponentInfo(Component::Controller(id_bytes)) => {
                let controller_id = PeerId::from_bytes(&id_bytes[..]).unwrap();
                println!("Controller PeerId: {:?}", controller_id.to_string());
            }
            InteractiveMessage::ComponentInfo(Component::Analyzer(id_bytes)) => {
                let analyzer_id = PeerId::from_bytes(&id_bytes[..]).unwrap();
                println!("\nAnalyzer is: {:?}", analyzer_id.to_string());
                self.add_analyzer(analyzer_id);
            }
            InteractiveMessage::CompletedTest(test_item) => {
                println!("{:?}", test_item);
            }
            _ => {}
        };
    }

    pub fn run_from(&mut self, args: Vec<String>) {
        match clap_Command::try_get_matches_from(CMD.to_owned(), args.clone()) {
            Ok(matches) => {
                self.cmd_match(&matches);
            }
            Err(err) => {
                err.print().expect("Error writing Error");
            }
        };
    }

    pub fn cmd_match(&mut self, matches: &ArgMatches) {
        // let matches = self.client().arg_matches();

        // if let Some(ref matches) = matches.subcommand_matches("requestsample") {
        //     if let Some(_) = matches.subcommand_matches("baidu") {
        //         let rt = tokio::runtime::Runtime::new().unwrap();
        //         let async_req = async { println!("测试成功") };
        //         rt.block_on(async_req);
        //     };
        // }

        if let Some(ref matches) = matches.subcommand_matches("init") {
            println!("\nBFT测试平台初始化成功！");
            self.init();
        }

        if let Some(ref matches) = matches.subcommand_matches("printUnfinishedTestItems") {
            // println!("\nBFT测试平台初始化成功！");
            self.print_unfinished_test_items();
        }

        if let Some(ref matches) = matches.subcommand_matches("test") {
            if let Some(_) = matches.subcommand_matches("pbft") {
                println!("\n开始进行PBFT共识协议的测试");
            };

            if let Some(_) = matches.subcommand_matches("hotstuff") {
                
            };

            if let Some(_) = matches.subcommand_matches("chain_hotstuff") {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let async_req =
                    async { println!("开始进行chain_hotstuff共识协议的测试") };
                rt.block_on(async_req);
            };
        }
    }

    // Framework network message handler startup function
    pub async fn message_handler_start(&mut self) {
        // let mut stdin = io::BufReader::new(io::stdin()).lines();

        loop {
            tokio::select! {
                Some(args) = self.args_recevier.recv() => {
                    // println!("{:?}", &args);
                    self.run_from(args);
                }
                // line = stdin.next_line() => {
                //     if let Ok(Some(command)) = line {
                //         println!("{}", command);
                //         let count = command.parse::<usize>().ok();
                //         match count {
                //             None => match &command as &str {
                //                 "Init" => {
                //                     self.init();
                //                 }
                //                 "AssignKey" => {
                //                     self.assign_key_to_consensus_node();
                //                 }
                //                 "NextTestItem" => {
                //                     self.configure_analyzer();
                //                 }
                //                 "StartTest" => {
                //                     self.start_test();
                //                 }
                //                 "ProtocolStart" => {}
                //                 _ => {
                //                     println!("Command is not found!");
                //                 }
                //             },
                //             Some(count) => {
                //                 self.make_consensus_requests(count);
                //             }
                //         }
                //     }
                // }
                event = self.peer.network_swarm_mut().select_next_some() => match event {
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        let peer_id = PeerId::from_bytes(&message.source[..]).unwrap();
                        if self.analyzer_id().to_string().eq(&peer_id.to_string()) {
                            let message: Message = coder::deserialize_for_bytes(&message.data);
                            self.analyzer_message_handler(message);
                            
                        } else if self.connected_nodes.contains_key(&peer_id.to_string()) {
                            
                        }

                        // let msg: CommandMessage = coder::deserialize_for_bytes(&message.data);
                        // match msg.command {
                        //     // MessageType::Reply(_) => {
                        //     //     self.executor.message_handler(&self.id.to_bytes(), &message.data).await;
                        //     // }
                        //     _ => {}
                        // }
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
                                println!("\nController PeerId: {:?}", controller_id.to_string());
                            }
                            InteractiveMessage::ComponentInfo(Component::Analyzer(id_bytes)) => {
                                let analyzer_id = PeerId::from_bytes(&id_bytes[..]).unwrap();
                                println!("\nAnalyzer is: {:?}", analyzer_id.to_string());
                                self.add_analyzer(analyzer_id);
                            },
                            _ => {}
                        };
                        //self.executor.message_handler(&self.id.to_bytes(), &message.data).await;
                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                        let swarm = self.peer.network_swarm_mut();
                        for (peer, _) in list {
                            println!("Discovered {:?}", &peer);
                            swarm.behaviour_mut().unicast.add_node_to_partial_view(&peer);
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
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
