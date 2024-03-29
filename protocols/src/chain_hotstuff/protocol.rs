use components::{
    behaviour::{self, PhaseState, ProtocolBehaviour, SendType},
    message::Request,
};

use crate::chain_hotstuff::common::{get_generic_hash, GENERIC};
use crate::chain_hotstuff::{
    common::{generate_bls_keys, get_request_hash},
    message::{Block, ConsensusMessage, ConsensusNodePKInfo, Generic, MessageType, Vote, QC},
};
use blsttc::SignatureShare;
use chrono::Local;
use libp2p::PeerId;
use std::thread::sleep;
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tokio::sync::{Mutex, Notify};
use utils::{coder, crypto::threshold_blsttc::TBLSKey};

use super::{common::get_block_hash, log::Log, state::State};
use crate::chain_hotstuff::message::Mode;

pub struct ChainHotstuffProtocol {
    pub state: State,
    pub log: Log,
    pub analyzer_id: PeerId,
    pub phase_map: HashMap<u8, String>,
    pub consensus_nodes: HashMap<String, PeerId>,
    pub pk_consensus_nodes: HashMap<u64, Vec<u8>>,
    pub consensus_node_pks: HashMap<Vec<u8>, ConsensusNodePKInfo>,

    pub keypair: Option<TBLSKey>,
    pub peer_id: String,

    pub taken_requests: HashSet<Vec<u8>>,
    pub request_buffer: Vec<String>,
    pub request_buffer_map: HashMap<String, (Vec<u8>, usize)>,

    // Current view timeout notification
    pub view_timeout_notify: Arc<Notify>,
    // Current view timeout check stop notification
    pub view_timeout_stop_notify: Arc<Notify>,
    pub consensus_notify: Arc<Notify>,
}

impl Default for ChainHotstuffProtocol {
    fn default() -> Self {
        Self {
            state: State::new(),
            log: Log::new(),

            analyzer_id: PeerId::random(),
            phase_map: HashMap::new(),

            consensus_nodes: HashMap::new(),
            keypair: None,
            peer_id: "".to_string(),

            request_buffer: Vec::new(),
            request_buffer_map: HashMap::new(),
            pk_consensus_nodes: HashMap::new(),
            consensus_node_pks: HashMap::new(),

            view_timeout_notify: Arc::new(Notify::new()),
            view_timeout_stop_notify: Arc::new(Notify::new()),
            consensus_notify: Arc::new(Notify::new()),
            taken_requests: HashSet::new(),
        }
    }
}

impl ChainHotstuffProtocol {
    // storage TBLS keypair
    pub fn storage_tbls_key(&mut self, current_peer_id: PeerId, tbls_key: &TBLSKey) {
        self.keypair = Some(tbls_key.clone());
    }

    //storage other consensus node's public key and number
    pub fn storage_consensus_node_pk(
        &mut self,
        pks_info: &HashMap<Vec<u8>, ConsensusNodePKInfo>,
        current_peer_id: PeerId,
    ) -> PhaseState {
        let mut send_query = VecDeque::new();
        println!("Enter storage_consensus_node_pk");
        for (peer_id, pk_info) in pks_info {
            let id = PeerId::from_bytes(peer_id).unwrap();
            println!("【【【【{}】】】】", id.to_string());
            println!("{:#?}", pk_info);

            self.pk_consensus_nodes
                .insert(pk_info.number, peer_id.to_vec());
            println!("Consensus nodes count: {}", self.pk_consensus_nodes.len());
        }

        self.next_view(current_peer_id);
        PhaseState::OverMessage(None, send_query)
    }

    // change current leader
    pub fn change_leader(&mut self, view_num: u64) {
        println!("Enter change leader");
        let node_count = self.pk_consensus_nodes.len();
        let leader_num = view_num % node_count as u64;
        let next_leader_num = (view_num + 1) % node_count as u64;

        self.state.current_leader = self.pk_consensus_nodes.get(&leader_num).unwrap().to_vec();
        self.state.next_leader = self
            .pk_consensus_nodes
            .get(&next_leader_num)
            .unwrap()
            .to_vec();
        println!("next leader:{:?}", self.state.next_leader.clone());
    }

    // next view
    pub fn next_view(&mut self, current_peer_id: PeerId) -> PhaseState {
        let mut send_query = VecDeque::new();
        println!("Enter next view");
        // get next leader
        let next_view = self.state.view + 1;
        self.change_leader(next_view);

        match self.state.current_leader.cmp(&current_peer_id.to_bytes()) {
            Ordering::Equal => {
                self.state.view = next_view;
                println!("leader view changed success");
                return PhaseState::ContinueExecute(send_query);
            }
            _ => {
                // send generic message to next leader
                let generic = Generic {
                    view_num: self.state.view,
                    block: None,
                    justify: self.state.generic_qc.clone(),
                    from_peer_id: current_peer_id.to_bytes(),
                };

                let generic_msg = ConsensusMessage {
                    msg_type: MessageType::Generic(generic.clone()),
                };

                let serialized_msg = coder::serialize_into_bytes(&generic_msg);

                //根据generic进行包装

                if let None = generic.block {
                    let leader_id = PeerId::from_bytes(&self.state.current_leader)
                        .expect("Leader peer id error.");
                    send_query.push_back(SendType::Unicast(leader_id, serialized_msg.clone()));
                } else {
                    send_query.push_back(SendType::Broadcast(serialized_msg.clone()));
                }

                println!("I have sent the Next View message");
            }
        }
        self.state.view = next_view;
        println!("********************************************");
        println!("Current view is : {:?}", self.state.view);
        println!("********************************************");

        let peer = PeerId::from_bytes(&self.state.current_leader).unwrap();
        println!("");
        println!("");
        println!("============================================");
        println!("Current leader: {}", peer.to_string());
        println!("============================================");
        println!("");
        println!("");

        // start new view timing
        self.view_timeout_start(next_view);
        PhaseState::ContinueExecute(send_query)
    }

    pub fn view_timeout_start(&self, view_num: u64) {
        println!("==============【view timeout start】==============");
        let timeout_notify = self.view_timeout_notify.clone();
        let stop_notify = self.view_timeout_stop_notify.clone();
        let current_timeout = Duration::from_secs(self.state.current_view_timeout);
        println!("Current view timeout: {}", self.state.current_view_timeout);

        tokio::spawn(async move {
            if let Err(_) = tokio::time::timeout(current_timeout, stop_notify.notified()).await {
                timeout_notify.notify_one();
                println!("View({}) is timeout!", view_num);
            }
        });
    }

    pub fn view_timeout_stop(&mut self) {
        self.state.current_view_timeout = self.state.tf;
        self.view_timeout_stop_notify.notify_one();
    }

    pub fn safe_node(&self, block: &Block, qc: &Option<QC>) -> bool {
        let qc = if let Some(qc) = qc {
            qc
        } else {
            return true;
        };

        if let Some(locked_qc) = &self.state.locked_qc {
            let locked_qc_block_hash = get_block_hash(&locked_qc.block);
            (qc.view_num > locked_qc.view_num) || block.parent_hash.eq(&locked_qc_block_hash)
        } else {
            (qc.view_num > 0) || block.parent_hash.eq("")
        }
    }

    fn handle_request(&mut self, request: &Request, current_peer_id: PeerId) -> PhaseState {
        self.taken_requests
            .insert(coder::serialize_into_bytes(request));
        let mut send_query = VecDeque::new();
        let request_hash = get_request_hash(&request);

        let count = self.request_buffer.len();
        self.request_buffer.insert(count, request_hash.clone());

        let parent_hash = if let Some(qc) = &self.state.high_qc {
            get_block_hash(&qc.block)
        } else {
            "".to_string()
        };

        let block = Block {
            cmd: request.cmd.clone(),
            parent_hash,
            justify: Box::new(self.state.high_qc.clone()),
        };

        // broadcast generic message
        let generic = Generic {
            view_num: self.state.view,
            block: Some(block),
            justify: None,
            from_peer_id: current_peer_id.to_bytes(),
        };
        let msg = ConsensusMessage {
            msg_type: MessageType::Generic(generic.clone()),
        };
        let serialized_msg = coder::serialize_into_bytes(&msg);

        //根据generic进行包装
        send_query.push_back(SendType::Broadcast(serialized_msg.clone()));

        println!("");
        println!("################");
        println!("# I am leader! #");
        println!("################");
        println!("");

        let data = self.handle_generic(&generic, current_peer_id);
        match data {
            PhaseState::ContinueExecute(msgs) => {
                for msg in msgs {
                    send_query.push_back(msg);
                }
                return PhaseState::ContinueExecute(send_query);
            }
            PhaseState::Complete(request, msgs) => {
                for msg in msgs {
                    send_query.push_back(msg);
                }
                return PhaseState::Complete(request, send_query);
            }
            _ => PhaseState::ContinueExecute(VecDeque::new()),
        }
    }

    fn handle_generic(&mut self, generic: &Generic, current_peer_id: PeerId) -> PhaseState {
        let mut send_query = VecDeque::new();
        if let None = &generic.block {
            let msg_list = self.handle_newview_generic(generic, current_peer_id);
            return msg_list;
        }
        let request = Request {
            cmd: generic.clone().block.unwrap().cmd,
        };
        self.taken_requests
            .insert(coder::serialize_into_bytes(&request));

        if self.state.view != generic.view_num {
            println!("current view: {:?}", self.state.view);
            println!("generic view: {:?}", generic.view_num);
            eprintln!("View is error!");
            return PhaseState::ContinueExecute(send_query);
        }

        let data = self.replica_handle_generic(generic, current_peer_id);
        data
    }

    //leader collects the vote message
    fn handle_vote(&mut self, vote: &Vote, current_peer_id: PeerId) -> PhaseState {
        // let three_secs = Duration::from_millis(400);
        // sleep(three_secs);
        let mut send_query = VecDeque::new();

        println!("");
        println!("*********************** Get the Vote *************************");
        println!("");

        let generic_msg_hash = get_generic_hash(GENERIC, vote.view_num, &vote.block);
        let pk_info = self.consensus_node_pks.get(&vote.from_peer_id).unwrap();

        // verify partial signature
        if !pk_info
            .public_key
            .verify(&vote.partial_signature, generic_msg_hash.as_bytes())
        {
            eprintln!("Partial signature is invalid!");
            return PhaseState::ContinueExecute(send_query);
        }

        self.log.record_messgae_partial_signature(
            &generic_msg_hash,
            pk_info.number,
            &vote.partial_signature,
        );

        let partial_sig_count = self
            .log
            .get_partial_signatures_count_by_message_hash(&generic_msg_hash);
        let threshold = 2 * self.state.fault_tolerance_count + 1;

        println!("Current partial signature is {}", partial_sig_count);
        if partial_sig_count != threshold as usize {
            return PhaseState::ContinueExecute(send_query);
        }
        let partial_sigs = self
            .log
            .get_partial_signatures_by_message_hash(&generic_msg_hash);

        let signature = self
            .keypair
            .as_ref()
            .unwrap()
            .combine_partial_signatures(partial_sigs);

        let mut qc = QC::new(vote.view_num, &vote.block);
        qc.set_signature(&signature);

        self.state.generic_qc = Some(qc);
        println!("All done");

        let cmd = vote.clone().block.cmd;
        let request = Request { cmd };
        let msg = coder::serialize_into_bytes(&request);

        return PhaseState::ContinueExecute(send_query);
    }

    pub fn handle_newview_generic(
        &mut self,
        generic: &Generic,
        current_peer_id: PeerId,
    ) -> PhaseState {
        let mut send_query = VecDeque::new();
        self.log.record_generic(generic.view_num, generic);
        let count = self.log.get_generics_count_by_view(generic.view_num);
        println!("count: {}", count);
        let threshold = 2 * self.state.fault_tolerance_count + 1;
        // 2f+1 newview, calculate highQC
        if count == threshold as usize {
            let high_qc = self
                .log
                .get_high_qc_by_view(generic.view_num, self.state.generic_qc.clone());
            println!("Cuurent high qc is : {:?}", high_qc.clone());
            self.state.high_qc = high_qc.clone();
            self.state.generic_qc = high_qc;

            self.next_view(current_peer_id);
            return PhaseState::Over(None);
        }
        PhaseState::ContinueExecute(send_query)
    }

    pub fn replica_handle_generic(
        &mut self,
        generic: &Generic,
        current_peer_id: PeerId,
    ) -> PhaseState {
        let mut send_query = VecDeque::new();
        println!("Enter replica handle generic");
        let b_1 = if let Some(b) = &generic.block {
            b
        } else {
            eprintln!("Generic block is not found.");
            return PhaseState::ContinueExecute(send_query);
        };

        if !self.safe_node(b_1, &b_1.justify) {
            eprintln!("Safenode authentication failed");
            return PhaseState::ContinueExecute(send_query);
        } else {
            if let Some(keypair) = &self.keypair {
                let sign_msg_hash = get_generic_hash(GENERIC, self.state.view, b_1);
                let partial_signature = keypair.sign(sign_msg_hash.as_bytes());

                let vote = Vote {
                    view_num: self.state.view,
                    block: b_1.clone(),
                    partial_signature: partial_signature,
                    from_peer_id: current_peer_id.to_bytes(),
                };

                let pk_info = self.consensus_node_pks.get(&vote.from_peer_id).unwrap();
                if let Ordering::Equal = self.state.next_leader.cmp(&current_peer_id.to_bytes()) {
                    self.log.record_messgae_partial_signature(
                        &sign_msg_hash,
                        pk_info.number,
                        &vote.partial_signature,
                    );
                }

                let vote_msg = ConsensusMessage {
                    msg_type: MessageType::Vote(vote.clone()),
                };

                let serialized_vote_msg = coder::serialize_into_bytes(&vote_msg);
                //根据generic进行包装

                println!("next leader is : {:?}", self.state.next_leader.to_vec());
                let next_leader_id =
                    PeerId::from_bytes(&self.state.next_leader).expect("Leader peer id error.");
                let current_leader = PeerId::from_bytes(&self.state.current_leader).expect("");
                let leader_id =
                    PeerId::from_bytes(&self.state.next_leader).expect("Leader peer id error.");
                if current_peer_id.to_bytes() == leader_id.to_bytes() {
                    println!("");
                    println!("");
                    println!("################");
                    println!("# I am the next leader!");
                    println!("################");
                    println!("");
                    println!("");
                } else {
                    println!("");
                    println!("");
                    println!("==================================================");
                    println!("Send vote to next leader({})!", next_leader_id.to_string());
                    println!("==================================================");
                    println!("");
                    println!("");

                    send_query.push_back(SendType::Unicast(leader_id, serialized_vote_msg));
                    let m2 = self.next_view(current_peer_id);
                    if let PhaseState::ContinueExecute(msg) = m2 {
                        for message in msg {
                            send_query.push_back(message)
                        }
                    };
                }
            } else {
                eprintln!("Keypair is not found!");
            }
            self.view_timeout_stop();
        }

        // start pre-commit phase on b_1's parent
        let b_2 = if let Some(qc) = &*b_1.justify {
            &qc.block
        } else {
            return PhaseState::ContinueExecute(send_query);
        };

        let b_2_hash = get_block_hash(b_2);
        if b_1.parent_hash.eq(&b_2_hash) {
            self.state.generic_qc = *b_1.justify.clone();
        } else {
            eprintln!("Not formed one-chain.");
            return PhaseState::ContinueExecute(send_query);
        }

        // start commit phase on b_1's grandparent
        let b_3 = if let Some(qc) = &*b_2.justify {
            &qc.block
        } else {
            return PhaseState::ContinueExecute(send_query);
        };

        let b_3_hash = get_block_hash(b_3);
        if b_2.parent_hash.eq(&b_3_hash) {
            self.state.locked_qc = *b_2.justify.clone();
        } else {
            eprintln!("Not formed two-chain.");
            return PhaseState::ContinueExecute(send_query);
        }

        // start decide phase on b_1's great-grandparent
        let b_4 = if let Some(qc) = &*b_3.justify {
            &qc.block
        } else {
            return PhaseState::ContinueExecute(send_query);
        };

        let b_4_hash = get_block_hash(&b_4);
        if b_3.parent_hash.eq(&b_4_hash) {
            let three_secs = Duration::from_secs(1);
            sleep(three_secs);
            println!("");
            println!("");
            println!("+++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
            println!("+  Execute new commands, current command is:");
            println!("+  {:?}", b_4.cmd);
            println!("+++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
            println!("");
            println!("");

            let request = Request {
                cmd: b_4.clone().cmd,
            };
            let msg = ConsensusMessage {
                msg_type: MessageType::Request(request.clone()),
            };
            let serialized_data = coder::serialize_into_bytes(&msg);
            // self.taken_requests
            //     .remove(&coder::serialize_into_bytes(&request));
            println!("My id is : {:?}", current_peer_id.to_string());
            return PhaseState::Complete(request, send_query);
        } else {
            eprintln!("Not formed three-chain.");
            return PhaseState::ContinueExecute(send_query);
        }
    }

    // pub fn handle_end(&mut self, block: &Block) -> PhaseState {
    //     let mut send_query = VecDeque::new();
    //     let request = Request {
    //         cmd: block.cmd.clone(),
    //     };
    //     let data = ConsensusMessage {
    //         msg_type: MessageType::Request(request),
    //     };
    //     let message = coder::serialize_into_bytes(&data);
    //     msg_list.push(ProtocolMessage::ConsensusMessage(Handler_data {
    //         is_end: ConsensusEnd::No,
    //         data: DataType::Delete(message),
    //     }));
    //     return msg_list;
    //     // let request_hash = get_request_hash(&request);
    //     // let request_info = self.request_buffer_map.get(&request_hash);
    //     // if let Some(_) = request_info {
    //     //     self.request_buffer.remove(0);
    //     //     self.request_buffer_map.remove(&request_hash);
    //     // }

    //     // let count = self.request_buffer.len();
    //     // if count == 0 {
    //     //     self.mode = Mode::Init;
    //     // } else {
    //     //     self.mode = Mode::NotIsLeader(count as u64);
    //     // }

    //     // println!("Current request buffer: {:?}", self.request_buffer);
    //     // println!("Current request buffer map: {:?}", self.request_buffer_map);
    // }

    pub fn distribute_keys(&mut self, current_peer_id: PeerId) -> PhaseState {
        let mut send_queue = VecDeque::new();
        println!("consensus_nodes:{:?}", &self.consensus_nodes.clone());
        let distribute_tbls_key_vec =
            generate_bls_keys(&self.consensus_nodes, self.state.fault_tolerance_count);

        println!("key count: {}", distribute_tbls_key_vec.len());
        let key = distribute_tbls_key_vec[0].tbls_key.clone();
        let key_msg = ConsensusMessage {
            msg_type: MessageType::TBLSKey(key),
        };
        let serialized_key = coder::serialize_into_bytes(&key_msg);
        println!("{:?}", serialized_key);
        let de_key: ConsensusMessage = coder::deserialize_for_bytes(&serialized_key);
        println!("{:#?}", de_key);

        let mut consensus_node_pks: HashMap<Vec<u8>, ConsensusNodePKInfo> = HashMap::new();
        for key_info in distribute_tbls_key_vec {
            let msg = ConsensusMessage {
                msg_type: MessageType::TBLSKey(key_info.tbls_key.clone()),
            };
            if (key_info.peer_id.clone().to_string().as_str() == self.peer_id.as_str()) {
                println!("This is my key!");
                self.storage_tbls_key(
                    PeerId::from_str(self.peer_id.as_str()).expect("msg"),
                    &key_info.tbls_key.clone(),
                );
            }

            let serialized_msg = coder::serialize_into_bytes(&msg);
            let data = SendType::Unicast(key_info.peer_id.clone(), serialized_msg);

            send_queue.push_back(data);

            let db_key = key_info.peer_id.clone().to_bytes();
            let consensus_node_pk_info = ConsensusNodePKInfo {
                number: key_info.number,
                public_key: key_info.tbls_key.public_key,
            };
            consensus_node_pks.insert(db_key, consensus_node_pk_info);
        }

        self.consensus_node_pks = consensus_node_pks.clone();

        let msg = ConsensusMessage {
            msg_type: MessageType::ConsensusNodePKsInfo(consensus_node_pks.clone()),
        };
        let serialized_msg = coder::serialize_into_bytes(&msg);
        let data = SendType::Broadcast(serialized_msg);
        send_queue.push_back(data);
        let data = self.storage_consensus_node_pk(&consensus_node_pks, current_peer_id.clone());
        if let PhaseState::ContinueExecute(msg) = data {
            for i in msg {
                send_queue.push_back(i);
            }
        }
        return PhaseState::OverMessage(None, send_queue);
    }
}

impl ProtocolBehaviour for ChainHotstuffProtocol {
    fn init_timeout_notify(&mut self, timeout_notify: Arc<Notify>) {
        self.view_timeout_notify = timeout_notify;
    }

    fn extra_initial_start(
        &mut self,
        consensus_nodes: HashSet<PeerId>,
        current_peer_id: Vec<u8>,
    ) -> PhaseState {
        let mut map: HashMap<String, PeerId> = HashMap::new();
        consensus_nodes.iter().for_each(|i| {
            map.insert(i.to_string(), *i);
        });
        self.consensus_nodes = map;
        self.peer_id = PeerId::from_bytes(&current_peer_id.clone())
            .expect("Found invalid UTF-8")
            .to_string();
        // self.analyzer_id = PeerId::from_str(analyzer_id.as_str()).expect("error");
        let data =
            self.distribute_keys(PeerId::from_bytes(&current_peer_id.clone()).expect("Error"));
        return data;
    }

    fn is_leader(&self, peer_id: Vec<u8>) -> bool {
        match self.state.current_leader.cmp(&peer_id) {
            Ordering::Equal => {
                println!("是leader");
                true
            }
            _ => false,
        }
    }

    fn generate_serialized_request_message(&self, request: &Request) -> Vec<u8> {
        let msg = ConsensusMessage {
            msg_type: MessageType::Request(request.to_owned()),
        };
        let data = coder::serialize_into_bytes(&msg);
        data
    }

    fn consensus_protocol_message_handler(
        &mut self,
        _msg: &[u8],
        current_peer_id: Vec<u8>,
        peer_id: Option<PeerId>,
    ) -> PhaseState {
        if let Some(peer_id) = peer_id {
            println!("Get message from : {:?}", peer_id);
        }

        let message: ConsensusMessage = coder::deserialize_for_bytes(_msg);
        match message.msg_type {
            MessageType::Request(request) => {
                println!("");
                println!("*********************** Get the Request *************************");
                println!("");
                return self
                    .handle_request(&request, PeerId::from_bytes(&current_peer_id).expect("msg"));
            }
            MessageType::Generic(generic) => {
                println!("");
                println!("*********************** Get the Generic *************************");
                println!("");
                return self
                    .handle_generic(&generic, PeerId::from_bytes(&current_peer_id).expect("msg"));
            }
            MessageType::Vote(vote) => {
                return self.handle_vote(&vote, PeerId::from_bytes(&current_peer_id).expect("msg"));
            }

            MessageType::TBLSKey(key) => {
                println!("Received TBLSKey!");
                self.storage_tbls_key(PeerId::from_bytes(&current_peer_id).expect("msg"), &key);
            }
            MessageType::ConsensusNodePKsInfo(msg) => {
                println!("Received ConsensusNodePKsInfo!");
                self.consensus_node_pks = msg.clone();
                return self.storage_consensus_node_pk(
                    &msg,
                    PeerId::from_bytes(&current_peer_id).expect("msg"),
                );
            }
            _ => {}
        }

        let mut queue = VecDeque::new();
        queue.push_back(SendType::Broadcast(vec![]));
        PhaseState::ContinueExecute(queue)
    }

    fn receive_consensus_requests(&mut self, requests: Vec<Request>) {
        todo!()
    }

    fn phase_map(&self) -> HashMap<u8, String> {
        HashMap::new()
    }

    fn get_current_phase(&mut self, _msg: &[u8]) -> u8 {
        1
    }

    // fn protocol_phases(&mut self) -> HashMap<u8, Vec<u8>> {
    //     let mut hash_map:HashMap<u8,Vec<u8>> = HashMap::new();
    //     println!("No security test interface is implemented！");
    //     let block = Block {
    //         cmd: "test".to_string(),
    //         parent_hash: "test".to_string(),
    //         justify: Box::new(None),
    //     };

    //     let qc = QC {
    //         view_num: 1,
    //         block: block.clone(),
    //         signature: None,
    //     };

    //     let generic = Generic {
    //         view_num: 1,
    //         block: Some(block),
    //         justify: Some(qc),
    //         from_peer_id: vec![],
    //     };
       
    //     let vote = Vote {
    //         view_num: 1,
    //         block: block.clone(),
    //         partial_signature: None,
    //         from_peer_id: vec![1],
    //     };
        
        

    //     let generic_json_bytes = coder::serialize_into_json_str(&generic).as_bytes().to_vec();
    //     let vote_json_bytes = coder::serialize_into_json_str(&vote).as_bytes().to_vec();
        
    //     hash_map.insert(1, generic_json_bytes.clone());
    //     hash_map.insert(2, vote_json_bytes.clone());
        

    //     self.phase_map.insert(1, String::from("Generic"));
    //     self.phase_map.insert(2, String::from("Vote"));
        
    //     hash_map
    // }

    fn protocol_reset(&mut self) {
        self.view_timeout_stop();
        self.state.view = 0;
        self.state.high_qc = None;
        self.state.generic_qc = None;
        self.state.locked_qc = None;
        self.state.current_leader = vec![];
        self.state.next_leader = vec![];
        self.state.node_count = 4;
        self.state.fault_tolerance_count = 1;
        self.state.tf = 10;
        self.state.current_view_timeout = 10;
        self.keypair = None;
        self.consensus_nodes = HashMap::new();
        self.pk_consensus_nodes = HashMap::new();
        self.consensus_node_pks = HashMap::new();
        self.log.message_signatures = HashMap::new();
        self.log.generics = HashMap::new();
        self.peer_id = PeerId::random().to_string();
        self.taken_requests = HashSet::new();
        self.request_buffer = vec![];
        self.request_buffer_map = HashMap::new();

        self.view_timeout_notify = Arc::new(Notify::new());
        self.view_timeout_stop_notify = Arc::new(Notify::new());
    }

    fn view_timeout_handler(&mut self, current_peer_id: PeerId) -> PhaseState {
        let current_view_timeout = self.state.current_view_timeout;
        self.state.current_view_timeout = current_view_timeout * 2;

        // next view
        let msg = self.next_view(current_peer_id.clone());
        return msg;
    }

    fn check_taken_request(&self, request: Vec<u8>) -> bool {
        if self.taken_requests.contains(&request) {
            true
        } else {
            false
        }
    }

    fn current_request(&self) -> Request {
        self.state.current_request.clone()
    }

    fn set_current_request(&mut self, request: &Request) {
        self.state.current_request = request.clone();
    }

    // fn receive_consensus_requests(&mut self, _requests: Vec<Request>) {
    //     todo!()
    // }
}
