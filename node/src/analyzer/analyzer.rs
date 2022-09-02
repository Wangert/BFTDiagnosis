use std::{collections::HashMap, error::Error, sync::Arc};

use cli::{client::Client, cmd::rootcmd::ANALYZER_CMD};
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

use tokio::{io::{self, AsyncBufReadExt}, sync::Notify, sync::mpsc::{Sender, Receiver, self}};
use utils::coder::{self, serialize_into_bytes};

use crate::{
    common::{generate_bls_keys, generate_consensus_requests_command},
    message::{
        Command, CommandMessage, Component, ConsensusData, ConsensusDataMessage, ConsensusEndData,
        ConsensusNodePKInfo, ConsensusStartData, InteractiveMessage, Message, TestItem,
    },
};

use clap::{Command as clap_Command, ArgMatches};

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
    // a data warehouse where protocol running data is stored and analyzed
    data_warehouse: DataWarehouse,

    completed_test_notify: Arc<Notify>,

    // client: Client,
    args_sender: Sender<Vec<String>>,
    args_recevier: Receiver<Vec<String>>,
}

impl Analyzer {
    pub fn new(peer: Peer, controller_id: PeerId, port: &str) -> Self {
        // let db_path = format!("./storage/data/{}_public_keys", port);
        let (args_sender, args_recevier) = mpsc::channel::<Vec<String>>(10);

        // let matches = ANALYZER_CMD.clone().get_matches();
        Self {
            id: peer.id,
            peer,
            connected_nodes: HashMap::new(),
            controller_id,
            other_analyzer_node: Vec::new(),
            current_test_item: None,
            data_warehouse: DataWarehouse::new(),
            completed_test_notify: Arc::new(Notify::new()),

            // client: Client::new(matches),
            args_sender,
            args_recevier,
        }
    }

    pub async fn peer_start(&mut self) -> Result<(), Box<dyn Error>> {
        self.peer_mut().swarm_start(false).await?;

        // let arg_sender = self.args_sender.clone();
        // self.client().run(arg_sender, 2);
        self.message_handler_start().await;

        Ok(())
    }

    pub fn id(&self) -> PeerId {
        self.id.clone()
    }

    pub fn id_bytes(&self) -> Vec<u8> {
        self.id.to_bytes()
    }

    pub fn peer_mut(&mut self) -> &mut Peer {
        &mut self.peer
    }

    pub fn controller_id(&self) -> PeerId {
        self.controller_id.clone()
    }

    // pub fn client(&self) -> Client {
    //     self.client.clone()
    // }

    pub fn args_sender(&self) -> Sender<Vec<String>> {
        self.args_sender.clone()
    }

    pub fn add_controller(&mut self, controller_id: PeerId) {
        self.controller_id = controller_id;
    }

    pub fn set_test_item(&mut self, test_item: TestItem) {
        self.current_test_item = Some(test_item);
    }

    pub fn current_test_item(&self) -> Option<TestItem> {
        self.current_test_item.clone()
    }

    pub fn data_warehouse(&mut self) -> &mut DataWarehouse {
        &mut self.data_warehouse
    }

    pub fn completed_test_notify(&self) -> Arc<Notify> {
        self.completed_test_notify.clone()
    }

    pub fn init(&mut self) {
        let id_bytes = self.id_bytes();
        let component = Component::Analyzer(id_bytes);
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

    pub fn start_test(&mut self) {
        if let Some(test_item) = self.current_test_item() {
            let data_warehouse = self.data_warehouse();
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
                TestItem::Scalability(_, _) => {}
                TestItem::Crash(_) => {}
                TestItem::Malicious(_) => {}
            }
        } else {
            println!("The current test item is empty. Please configure the test item!");
        };
    }

    pub fn next_test(&mut self) {
        let current_test_item = self.current_test_item().unwrap();

        let message = Message {
            interactive_message: InteractiveMessage::CompletedTest(current_test_item),
        };
        let serialized_message = coder::serialize_into_bytes(&message);
        let controller_id = self.controller_id();
        self.peer_mut()
            .network_swarm_mut()
            .behaviour_mut()
            .unicast
            .send_message(&controller_id, serialized_message);
    }

    pub fn controller_message_handler(&mut self, message: Message) {
        match message.interactive_message {
            InteractiveMessage::ComponentInfo(Component::Controller(id_bytes)) => {
                let controller_id = PeerId::from_bytes(&id_bytes[..]).unwrap();
                println!("Controller PeerId: {:?}", controller_id.to_string());
                self.add_controller(controller_id);
            }
            InteractiveMessage::ComponentInfo(Component::Analyzer(id_bytes)) => {}
            InteractiveMessage::TestItem(item) => {
                self.set_test_item(item);
            }
            InteractiveMessage::StartTest => {
                self.start_test();
            }
            _ => {}
        };
    }

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

    pub fn run_from(&mut self, args: Vec<String>) {
        match clap_Command::try_get_matches_from(ANALYZER_CMD.to_owned(), args.clone()) {
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


        if let Some(ref matches) = matches.subcommand_matches("init") {
            println!("BFT测试平台初始化成功！");
            self.init();
        }

        if let Some(ref matches) = matches.subcommand_matches("test") {
            if let Some(_) = matches.subcommand_matches("pbft") {
                println!("开始进行PBFT共识协议的测试");
            };

            if let Some(_) = matches.subcommand_matches("hotstuff") {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let async_req = async { println!("开始进行hotstuff共识协议的测试") };
                rt.block_on(async_req);
            };

            if let Some(_) = matches.subcommand_matches("chain_hotstuff") {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let async_req =
                    async { println!("开始进行chain_hotstuff共识协议的测试") };
                rt.block_on(async_req);
            };
        }
    }

    pub async fn message_handler_start(&mut self) {
        // let mut stdin = io::BufReader::new(io::stdin()).lines();

        let completed_test_notify = self.completed_test_notify();
        loop {
            tokio::select! {
                Some(args) = self.args_recevier.recv() => {
                    self.run_from(args);
                },
                // line = stdin.next_line() => {
                //     if let Ok(Some(command)) = line {
                //         println!("{}", command);
                //         let count = command.parse::<usize>().ok();
                //         match count {
                //             None => match &command as &str {
                //                 "Init" => {
                //                     self.init();
                //                 }
                //                 _ => {}
                //             },
                //             _ => {}
                //         }
                //     }
                // },
                _ = completed_test_notify.notified() => {
                    self.next_test();
                },
                event = self.peer.network_swarm_mut().select_next_some() => match event {
                    SwarmEvent::Behaviour(OutEvent::Unicast(UnicastEvent::Message(message))) => {
                        let peer_id = PeerId::from_bytes(&message.source[..]).unwrap();
                        if self.controller_id().to_string().eq(&peer_id.to_string()) {
                            let message: Message = coder::deserialize_for_bytes(&message.data);
                            self.controller_message_handler(message);
                        } else if self.connected_nodes.contains_key(&peer_id.to_string()) {
                            let consensus_data_message: ConsensusDataMessage = coder::deserialize_for_bytes(&message.data);
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
                            InteractiveMessage::ComponentInfo(Component::Analyzer(id_bytes)) => {

                            },
                            _ => {}
                        };

                    }
                    SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
                        let swarm = self.peer_mut().network_swarm_mut();
                        for (peer, _) in list {
                            println!("Discovered {:?}", &peer);
                            swarm.behaviour_mut().unicast.add_node_to_partial_view(&peer);
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                            // self.connected_nodes.insert(peer.to_string(), peer.clone());
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
                        println!("Listening on {:?}", address);
                    }
                    _ => {}
                }
            }
        }
    }
}
