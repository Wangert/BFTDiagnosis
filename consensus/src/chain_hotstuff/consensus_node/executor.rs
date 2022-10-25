use std::{cmp::Ordering, collections::HashMap, sync::Arc, time::Duration};

use libp2p::PeerId;
use storage::database::LevelDB;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Notify,
};
use utils::{
    coder::{self},
    crypto::threshold_blsttc::TBLSKey,
};

use crate::chain_hotstuff::consensus_node::state::Mode;

use super::{
    common::{get_block_hash, get_generic_hash, get_request_hash, GENERIC},
    log::Log,
    message::{Block, ConsensusNodePKInfo, Generic, Message, MessageType, Request, Vote, QC},
    state::State,
};

// Consensus node executor
pub struct Executor {
    // Record node's state information
    pub state: State,
    // Local logs
    pub log: Log,
    // Database
    pub db: Box<LevelDB>,
    // Node's keypair
    pub keypair: Option<TBLSKey>,
    // Executor and Node message transfer channel
    pub msg_tx: Sender<Vec<u8>>,
    pub msg_rx: Receiver<Vec<u8>>,
    // A new round of consensus notification
    pub consensus_notify: Arc<Notify>,
    // Current view timeout notification
    pub view_timeout_notify: Arc<Notify>,
    // Current view timeout check stop notification
    pub view_timeout_stop_notify: Arc<Notify>,
    // Current network's consensus nodes: number -> peer_id_vec
    pub consensus_nodes: HashMap<u64, Vec<u8>>,
}

impl Executor {
    pub fn new(db_path: &str) -> Executor {
        let (msg_tx, msg_rx) = mpsc::channel::<Vec<u8>>(10);

        Executor {
            state: State::new(),
            log: Log::new(),
            db: Box::new(LevelDB::new(db_path)),
            keypair: None,
            msg_tx,
            msg_rx,
            consensus_notify: Arc::new(Notify::new()),
            view_timeout_notify: Arc::new(Notify::new()),
            view_timeout_stop_notify: Arc::new(Notify::new()),
            consensus_nodes: HashMap::new(),
        }
    }

    //
    pub async fn message_handler(&mut self, current_peer_id: &[u8], msg: &[u8]) {
        let message: Message = coder::deserialize_for_bytes(msg);
        match message.msg_type {
            MessageType::Request(msg) => {
                self.handle_request(&msg, current_peer_id).await;
            }
            MessageType::Generic(msg) => {
                self.handle_generic(&msg, current_peer_id).await;
            }
            MessageType::Vote(msg) => {
                self.handle_vote(&msg, current_peer_id).await;
            }
            MessageType::TBLSKey(tbls_key) => {
                self.storage_tbls_key(current_peer_id, &tbls_key);
            }
            MessageType::ConsensusNodePKsInfo(pks_info) => {
                self.storage_consensus_node_pk(&pks_info, current_peer_id)
                    .await;
            }
            _ => {}
        }
    }

    // storage TBLS keypair
    pub fn storage_tbls_key(&mut self, current_peer_id: &[u8], tbls_key: &TBLSKey) {
        self.keypair = Some(tbls_key.clone());
        let peer_id = PeerId::from_bytes(current_peer_id).unwrap();
        println!("【【【【{}】】】】", peer_id.to_string());
        println!("{:#?}", self.keypair)
    }

    // storage other consensus node's public key and number
    pub async fn storage_consensus_node_pk(
        &mut self,
        pks_info: &HashMap<Vec<u8>, ConsensusNodePKInfo>,
        current_peer_id: &[u8],
    ) {
        for (peer_id, pk_info) in pks_info {
            let serialized_pk_info = coder::serialize_into_bytes(pk_info);
            self.db.write(peer_id, &serialized_pk_info);
            let value = self.db.read(peer_id).expect("read error");
            let de_value: ConsensusNodePKInfo = coder::deserialize_for_bytes(&value);

            let id = PeerId::from_bytes(peer_id).unwrap();
            println!("【【【【{}】】】】", id.to_string());
            println!("{:#?}", de_value);

            self.consensus_nodes
                .insert(pk_info.number, peer_id.to_vec());
            println!("Consensus nodes count: {}", self.consensus_nodes.len());
        }

        self.next_view(current_peer_id).await;
    }

    // get public key information by peer id
    pub fn get_public_key_info_by_peer_id(&self, peer_id: &[u8]) -> ConsensusNodePKInfo {
        let deserialized_pk_info = self.db.read(peer_id).expect("read pk info error!");
        let pk_info: ConsensusNodePKInfo = coder::deserialize_for_bytes(&deserialized_pk_info);

        pk_info
    }

    // change current leader
    pub fn change_leader(&mut self, view_num: u64) {
        let node_count = self.consensus_nodes.len();
        let leader_num = view_num % node_count as u64;
        let next_leader_num = (view_num + 1) % node_count as u64;

        self.state.current_leader = self.consensus_nodes.get(&leader_num).unwrap().to_vec();
        self.state.next_leader = self.consensus_nodes.get(&next_leader_num).unwrap().to_vec();
    }

    // View timeout timer start function
    pub fn view_timeout_start(&self, view_num: u64) {
        println!("==============【view timeout start】==============");
        let timeout_notify = self.view_timeout_notify.clone();
        let stop_notify = self.view_timeout_stop_notify.clone();
        let current_timeout = Duration::from_secs(self.state.current_view_timeout);
        // let is_primary = self.state.is_primary.clone();
        let mode = self.state.mode.clone();
        println!("Current view timeout: {}", self.state.current_view_timeout);

        tokio::spawn(async move {
            if let Err(_) = tokio::time::timeout(current_timeout, stop_notify.notified()).await {
                let mode_value = *mode.lock().await;
                // println!("[Timeout]: Current mode: {:?}", mode_value);
                match mode_value {
                    Mode::Done(n) | Mode::Do(n) if n == 0 => {
                        *mode.lock().await = Mode::Init;
                    }
                    Mode::Done(n) | Mode::Do(n) if n != 0 => {
                        *mode.lock().await = Mode::NotIsLeader(n);
                    }
                    _ => {}
                }
                timeout_notify.notify_one();
                println!("View({}) is timeout!", view_num);
            }
        });
    }

    // Check if you can initiate a proposal
    pub fn proposal_state_check(&self) {
        println!("==============【Proposal state check】==============");
        // let is_primary = self.state.is_primary.clone();
        // let have_request = self.state.hava_request.clone();
        let mode = self.state.mode.clone();
        let consensus_notify = self.consensus_notify.clone();
        tokio::spawn(async move {
            loop {
                //println!("is_primary:{} ----- have_request:{}", *is_primary.lock().await, *have_request.lock().await);
                let mode_value = *mode.lock().await;
                match mode_value {
                    Mode::Do(n) if n > 0 => {
                        // println!("[Proposal]: Current mode:{:?}", mode_value);
                        consensus_notify.notify_one();
                        *mode.lock().await = Mode::Done(n - 1);
                    }
                    _ => {}
                }
            }
        });
    }

    // next view
    pub async fn next_view(&mut self, current_peer_id: &[u8]) {
        // get next leader
        let next_view = self.state.view + 1;
        self.change_leader(next_view);

        match self.state.current_leader.cmp(&current_peer_id.to_vec()) {
            Ordering::Equal => {}
            _ => {
                // send generic message to next leader
                let generic = Generic {
                    view_num: self.state.view,
                    block: None,
                    justify: self.state.generic_qc.clone(),
                    from_peer_id: current_peer_id.to_vec(),
                };

                let generic_msg = Message {
                    msg_type: MessageType::Generic(generic),
                };

                let serialized_msg = coder::serialize_into_bytes(&generic_msg);
                self.send_message(&serialized_msg).await;
            }
        }

        self.state.view = next_view;

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
    }

    pub async fn handle_request(&mut self, request: &Request, current_peer_id: &[u8]) {
        // verify request message signature

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
            from_peer_id: current_peer_id.to_vec(),
        };
        let msg = Message {
            msg_type: MessageType::Generic(generic.clone()),
        };
        let serialized_msg = coder::serialize_into_bytes(&msg);
        self.send_message(&serialized_msg).await;
        println!("");
        println!("");
        println!("################");
        println!("# I am leader! #");
        println!("################");
        println!("");
        println!("");
        // leader handle generic
        self.handle_generic(&generic, current_peer_id).await;
    }

    pub async fn handle_newview_generic(&mut self, generic: &Generic) {
        self.log.record_generic(generic.view_num, generic);

        let count = self.log.get_generics_count_by_view(generic.view_num);

        println!("count: {}", count);
        let threshold = 2 * self.state.fault_tolerance_count + 1;
        // 2f+1 newview, calculate highQC
        if count == threshold as usize {
            let high_qc = self
                .log
                .get_high_qc_by_view(generic.view_num, self.state.generic_qc.clone());
            self.state.high_qc = high_qc.clone();
            self.state.generic_qc = high_qc;

            let mode = self.state.mode.clone();
            let mode_value = *mode.lock().await;
            match mode_value {
                Mode::Init => {
                    *mode.lock().await = Mode::Do(0);
                }
                Mode::Do(n) | Mode::Done(n) | Mode::NotIsLeader(n) => {
                    *mode.lock().await = Mode::Do(n);
                }
            }
        }
    }

    pub async fn handle_generic(&mut self, generic: &Generic, current_peer_id: &[u8]) {
        // println!("{:?}", generic);
        // Verify prepare signature

        // Next Leader collects generic messages
        if let None = &generic.block {
            self.handle_newview_generic(generic).await;
            return;
        }

        if self.state.view != generic.view_num {
            eprintln!("View is error!");
            return;
        }

        self.replica_handle_generic(generic, current_peer_id).await;
    }

    pub async fn replica_handle_generic(
        &mut self,
        generic: &Generic,
        current_peer_id: &[u8],
    ) {
        // b^1
        let b_1 = if let Some(b) = &generic.block {
            b
        } else {
            eprintln!("Generic block is not found.");
            return;
        };

        // safenode
        // The safety rule to accept a proposal is the branch of m.node extends from the currently locked node lockedQC.node.
        // The liveness rule is the replica will accept m if m.justify has a higher view than the current lockedQC.
        // The predicate is true as long as either one of two rules holds
        if !self.safe_node(b_1, &b_1.justify) {
            eprintln!("Safenode authentication failed");
            return;
        } else {
            if let Some(keypair) = &self.keypair {
                let sign_msg_hash = get_generic_hash(GENERIC, self.state.view, b_1);
                let partial_signature = keypair.sign(sign_msg_hash.as_bytes());

                let vote = Vote {
                    view_num: self.state.view,
                    block: b_1.clone(),
                    partial_signature,
                    from_peer_id: current_peer_id.to_vec(),
                };

                let pk_info = self.get_public_key_info_by_peer_id(&vote.from_peer_id);
                if let Ordering::Equal = self.state.next_leader.cmp(&current_peer_id.to_vec()) {
                    self.log.record_messgae_partial_signature(
                        &sign_msg_hash,
                        pk_info.number,
                        &vote.partial_signature,
                    );
                }

                let vote_msg = Message {
                    msg_type: MessageType::Vote(vote),
                };

                let serialized_vote_msg = coder::serialize_into_bytes(&vote_msg);
                self.send_message(&serialized_vote_msg).await;

                let next_leader_id = PeerId::from_bytes(&self.state.next_leader).expect("Leader peer id error.");
                println!("");
                println!("");
                println!("==================================================");
                println!("Send vote to next leader({})!", next_leader_id.to_string());
                println!("==================================================");
                println!("");
                println!("");
            } else {
                eprintln!("Keypair is not found!");
                return;
            }

            self.view_timeout_stop();
            self.send_end_message(b_1).await;
        }

        // start pre-commit phase on b_1's parent
        let b_2 = if let Some(qc) = &*b_1.justify {
            &qc.block
        } else {
            return;
        };

        let b_2_hash = get_block_hash(b_2);
        if b_1.parent_hash.eq(&b_2_hash) {
            self.state.generic_qc = *b_1.justify.clone();
        } else {
            eprintln!("Not formed one-chain.");
            return;
        }

        // start commit phase on b_1's grandparent
        let b_3 = if let Some(qc) = &*b_2.justify {
            &qc.block
        } else {
            return;
        };

        let b_3_hash = get_block_hash(b_3);
        if b_2.parent_hash.eq(&b_3_hash) {
            self.state.locked_qc = *b_2.justify.clone();
        } else {
            eprintln!("Not formed two-chain.");
            return;
        }

        // start decide phase on b_1's great-grandparent
        let b_4 = if let Some(qc) = &*b_3.justify {
            &qc.block
        } else {
            return;
        };

        let b_4_hash = get_block_hash(&b_4);
        if b_3.parent_hash.eq(&b_4_hash) {
            println!("");
            println!("");
            println!("+++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
            println!("+  Execute new commands, current command is:");
            println!("+  {:?}", b_4.cmd);
            println!("+++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
            println!("");
            println!("");
        } else {
            eprintln!("Not formed three-chain.");
            return;
        }
    }

    // Leader collects the generic vote
    pub async fn handle_vote(&mut self, vote: &Vote, _current_peer_id: &[u8]) {
        // println!("Vote: {:?}", vote);

        let generic_msg_hash = get_generic_hash(GENERIC, vote.view_num, &vote.block);
        let pk_info = self.get_public_key_info_by_peer_id(&vote.from_peer_id);

        // verify partial signature
        if !pk_info
            .public_key
            .verify(&vote.partial_signature, generic_msg_hash.as_bytes())
        {
            eprintln!("Partial signature is invalid!");
            return;
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
            return;
        }

        println!("Generate signature!");
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

    pub async fn send_end_message(&self, block: &Block) {
        let request = Request {
            cmd: block.cmd.clone(),
        };
        let request_hash = get_request_hash(&request);
        let msg = Message {
            msg_type: MessageType::End(request_hash),
        };
        let serialized_msg = coder::serialize_into_bytes(&msg);
        self.send_message(&serialized_msg).await;
    }

    pub async fn send_message(&self, msg: &[u8]) {
        self.msg_tx.send(msg.to_vec()).await.unwrap();
    }

    pub fn view_timeout_stop(&mut self) {
        self.state.current_view_timeout = self.state.tf;
        self.view_timeout_stop_notify.notify_one();
    }
}

#[derive(Debug)]
pub enum NextView {
    True,
    False,
}
