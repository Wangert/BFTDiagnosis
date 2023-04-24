use std::{cmp::Ordering, collections::HashMap, sync::Arc, time::Duration};

use libp2p::PeerId;
use storage::database::LevelDB;
use tokio::{sync::{
    mpsc::{self, Receiver, Sender},
    Notify,
}, time};
use utils::{
    coder::{self},
    crypto::threshold_blsttc::TBLSKey,
};

use crate::basic_hotstuff::consensus_node::{
    common::{get_message_hash, get_request_hash, PREPARE, PRE_COMMIT},
    message::Vote,
    state::Mode,
};

use super::{
    common::{get_block_hash, COMMIT},
    log::Log,
    message::{
        Block, Commit, ConsensusNodePKInfo, Decide, Message, MessageType, NewView, PreCommit,
        Prepare, Request, QC,
    },
    state::State,
};

// Consensus node executor
pub struct Executor {
    pub state: State,
    pub log: Box<Log>,
    pub db: Box<LevelDB>,
    pub keypair: Option<TBLSKey>,
    pub msg_tx: Sender<Vec<u8>>,
    pub msg_rx: Receiver<Vec<u8>>,
    pub consensus_notify: Arc<Notify>,
    pub view_timeout_notify: Arc<Notify>,
    pub view_timeout_stop_notify: Arc<Notify>,
    pub consensus_nodes: HashMap<u64, Vec<u8>>,
}

impl Executor {
    pub fn new(db_path: &str) -> Executor {
        let (msg_tx, msg_rx) = mpsc::channel::<Vec<u8>>(10);

        Executor {
            state: State::new(),
            log: Box::new(Log::new()),
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

    pub async fn message_handler(&mut self, current_peer_id: &[u8], msg: &Vec<u8>) {
        let message: Message = coder::deserialize_for_bytes(msg);

        match message.msg_type {
            MessageType::Request(msg) => {
                self.handle_request(&msg, current_peer_id).await;
            }
            MessageType::NewView(msg) => {
                self.handle_newview(&msg).await;
            }
            MessageType::Prepare(msg) => {
                self.handle_prepare(&msg, current_peer_id).await;
            }
            MessageType::PrepareVote(msg) => {
                self.handle_prepare_vote(&msg, current_peer_id).await;
            }
            MessageType::PreCommit(msg) => {
                self.handle_precommit(&msg, current_peer_id).await;
            }
            MessageType::PreCommitVote(msg) => {
                self.handle_precommit_vote(&msg, current_peer_id).await;
            }
            MessageType::Commit(msg) => {
                self.handle_commit(&msg, current_peer_id).await;
            }
            MessageType::CommitVote(msg) => {
                self.handle_commit_vote(&msg, current_peer_id).await;
            }
            MessageType::Decide(msg) => {
                self.handle_decide(&msg, current_peer_id).await;
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

    pub fn storage_tbls_key(&mut self, current_peer_id: &[u8], tbls_key: &TBLSKey) {
        self.keypair = Some(tbls_key.clone());
        let peer_id = PeerId::from_bytes(current_peer_id).unwrap();
        println!("【【【【{}】】】】", peer_id.to_string());
        println!("{:#?}", self.keypair)
    }

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
            // println!("Consensus nodes: {:#?}", self.consensus_nodes);
        }

        self.newview(current_peer_id).await;
    }

    pub fn get_public_key_info_by_peer_id(&self, peer_id: &[u8]) -> ConsensusNodePKInfo {
        let deserialized_pk_info = self.db.read(peer_id).expect("read pk info error!");
        let pk_info: ConsensusNodePKInfo = coder::deserialize_for_bytes(&deserialized_pk_info);

        pk_info
    }

    pub fn change_leader(&mut self, view_num: u64) {
        let node_count = self.consensus_nodes.len();
        let leader_num = view_num % node_count as u64;

        self.state.primary = self.consensus_nodes.get(&leader_num).unwrap().to_vec();
    }

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

    pub async fn newview(&mut self, current_peer_id: &[u8]) {
        // get next leader
        let next_view = self.state.view + 1;
        self.change_leader(next_view);

        match self.state.primary.cmp(&current_peer_id.to_vec()) {
            Ordering::Equal => {}
            _ => {
                // send newview message to next leader
                let newview = NewView {
                    view_num: self.state.view,
                    justify: self.state.prepare_qc.clone(),
                    from_peer_id: current_peer_id.to_vec(),
                };

                let newview_msg = Message {
                    msg_type: MessageType::NewView(newview),
                };

                let serialized_msg = coder::serialize_into_bytes(&newview_msg);
                self.send_message(&serialized_msg).await;
            }
        }

        self.state.view = next_view;

        let peer = PeerId::from_bytes(&self.state.primary).unwrap();
        println!("Current leader: {}", peer.to_string());

        //self.state.mode = Mode::Waiting;
        // start new view timing
        self.view_timeout_start(next_view);
    }

    pub async fn handle_newview(&mut self, newview: &NewView) {
        self.log.record_newview(newview.view_num, newview);

        let count = self.log.get_newviews_count_by_view(newview.view_num);

        //println!("count: {}", count);
        let threshold = 2 * self.state.fault_tolerance_count + 1;
        // 2f+1 newview, calculate highQC
        if count == threshold as usize {
            let high_qc = self
                .log
                .get_high_qc_by_view(newview.view_num, self.state.prepare_qc.clone());
            self.state.high_qc = high_qc;

            // println!("qqqq");
            // execute a new round of consensus
            // self.consensus_notify.notify_one();
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

            // println!("Current mode: {:?}", *mode.lock().await);
            // *self.state.is_primary.lock().await = true;
        }
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
        };

        // broadcast prepare message
        let prepare = Prepare {
            view_num: self.state.view,
            block,
            justify: self.state.high_qc.clone(),
            from_peer_id: current_peer_id.to_vec(),
        };
        let msg = Message {
            msg_type: MessageType::Prepare(prepare),
        };
        let serialized_msg = coder::serialize_into_bytes(&msg);
        self.send_message(&serialized_msg).await;
    }

    pub async fn handle_prepare(&mut self, prepare: &Prepare, current_peer_id: &[u8]) {
        println!("{:#?}", prepare);
        // verify prepare signature

        let qc = prepare.justify.clone();
        if let None = qc {
            if prepare.block.parent_hash != "" {
                eprintln!("Current block does not extend from justify block.");
                return;
            }
        }

        if let Some(ref qc) = qc {
            let qc_block_hash = get_block_hash(&qc.block);
            if qc_block_hash.ne(&prepare.block.parent_hash) {
                eprintln!("Current block does not extend from justify block.");
                return;
            }

            // safenode
            // The safety rule to accept a proposal is the branch of m.node extends from the currently locked node lockedQC.node.
            // The liveness rule is the replica will accept m if m.justify has a higher view than the current lockedQC.
            // The predicate is true as long as either one of two rules holds
            if !self.safe_node(&prepare.block, qc) {
                eprintln!("Safenode authentication failed");
                return;
            }
        }

        self.state.primary = prepare.from_peer_id.clone();

        // vote
        let sign_msg_hash = get_message_hash(PREPARE, prepare.view_num, &prepare.block);
        if let Some(keypair) = &self.keypair {
            let partial_signature = keypair.sign(sign_msg_hash.as_bytes());

            let vote = Vote {
                view_num: prepare.view_num,
                block: prepare.block.clone(),
                partial_signature,
                from_peer_id: current_peer_id.to_vec(),
            };

            let vote_msg = Message {
                msg_type: MessageType::PrepareVote(vote),
            };

            let serialized_vote_msg = coder::serialize_into_bytes(&vote_msg);
            self.send_message(&serialized_vote_msg).await;
        } else {
            eprintln!("Keypair is not found!");
            return;
        }
    }

    // Leader collects the prepare vote
    pub async fn handle_prepare_vote(&mut self, vote: &Vote, _current_peer_id: &[u8]) {
        let prepare_msg_hash = get_message_hash(PREPARE, vote.view_num, &vote.block);
        let pk_info = self.get_public_key_info_by_peer_id(&vote.from_peer_id);

        // verify partial signature
        if !pk_info
            .public_key
            .verify(&vote.partial_signature, prepare_msg_hash.as_bytes())
        {
            eprintln!("Partial signature is invalid!");
            return;
        }

        self.log.record_messgae_partial_signature(
            &prepare_msg_hash,
            pk_info.number,
            &vote.partial_signature,
        );

        let partial_sig_count = self
            .log
            .get_partial_signatures_count_by_message_hash(&prepare_msg_hash);
        let threshold = 2 * self.state.fault_tolerance_count + 1;
        if partial_sig_count != threshold as usize {
            return;
        }

        let partial_sigs = self
            .log
            .get_partial_signatures_by_message_hash(&prepare_msg_hash);
        let signature = self
            .keypair
            .as_ref()
            .unwrap()
            .combine_partial_signatures(partial_sigs);

        let mut prepare_qc = QC::new(PREPARE, vote.view_num, &vote.block);
        prepare_qc.set_signature(&signature);

        // broadcast pre-commit message
        let pre_commit = PreCommit {
            view_num: vote.view_num,
            justify: Some(prepare_qc),
        };
        let pre_commit_msg = Message {
            msg_type: MessageType::PreCommit(pre_commit),
        };
        let serialized_msg = coder::serialize_into_bytes(&pre_commit_msg);
        self.send_message(&serialized_msg).await;

        println!("broadcast prepareQC!");
    }

    pub async fn handle_precommit(&mut self, pre_commit: &PreCommit, current_peer_id: &[u8]) {
        if let None = pre_commit.justify {
            eprintln!("【PreCommit】: Not found QC.");
            return;
        };
        let justify = pre_commit.justify.clone().unwrap();

        if let None = justify.signature {
            eprintln!("【PreCommit】: Not found signature.");
            return;
        }
        let signature = justify.signature.clone().unwrap();

        // match QC
        if justify.msg_type != PREPARE || justify.view_num != self.state.view {
            eprintln!("【PreCommit】: QC is invalid.");
            return;
        }
        // verify signature
        let prepare_msg_hash = get_message_hash(PREPARE, justify.view_num, &justify.block);
        if !self
            .keypair
            .as_ref()
            .expect("Keypair is not found!")
            .threshold_verify(&signature, prepare_msg_hash.as_bytes())
        {
            eprintln!("【PreCommit】: Signature is invalid.");
            return;
        }

        // set prepareQC
        self.state.prepare_qc = Some(justify.clone());

        // vote
        println!("Start precommit vote!");
        let sign_msg_hash = get_message_hash(PRE_COMMIT, pre_commit.view_num, &justify.block);
        if let Some(keypair) = &self.keypair {
            let partial_signature = keypair.sign(sign_msg_hash.as_bytes());

            let vote = Vote {
                view_num: pre_commit.view_num,
                block: justify.block.clone(),
                partial_signature,
                from_peer_id: current_peer_id.to_vec(),
            };

            let vote_msg = Message {
                msg_type: MessageType::PreCommitVote(vote),
            };

            let serialized_vote_msg = coder::serialize_into_bytes(&vote_msg);
            self.send_message(&serialized_vote_msg).await;
        } else {
            eprintln!("Keypair is not found!");
            return;
        }
    }

    // Leader collects the precommit vote
    pub async fn handle_precommit_vote(&mut self, vote: &Vote, _current_peer_id: &[u8]) {
        let precommit_msg_hash = get_message_hash(PRE_COMMIT, vote.view_num, &vote.block);
        let pk_info = self.get_public_key_info_by_peer_id(&vote.from_peer_id);

        // verify partial signature
        if !pk_info
            .public_key
            .verify(&vote.partial_signature, precommit_msg_hash.as_bytes())
        {
            eprintln!("Partial signature is invalid!");
            return;
        }

        self.log.record_messgae_partial_signature(
            &precommit_msg_hash,
            pk_info.number,
            &vote.partial_signature,
        );

        let partial_sig_count = self
            .log
            .get_partial_signatures_count_by_message_hash(&precommit_msg_hash);
        let threshold = 2 * self.state.fault_tolerance_count + 1;
        if partial_sig_count != threshold as usize {
            return;
        }

        let partial_sigs = self
            .log
            .get_partial_signatures_by_message_hash(&precommit_msg_hash);
        let signature = self
            .keypair
            .as_ref()
            .unwrap()
            .combine_partial_signatures(partial_sigs);

        let mut precommit_qc = QC::new(PRE_COMMIT, vote.view_num, &vote.block);
        precommit_qc.set_signature(&signature);

        // broadcast commit message
        let commit = Commit {
            view_num: vote.view_num,
            justify: Some(precommit_qc),
        };
        let commit_msg = Message {
            msg_type: MessageType::Commit(commit),
        };
        let serialized_msg = coder::serialize_into_bytes(&commit_msg);
        self.send_message(&serialized_msg).await;

        println!("broadcast precommitQC!");
    }

    pub async fn handle_commit(&mut self, commit: &Commit, current_peer_id: &[u8]) {
        if let None = commit.justify {
            eprintln!("【Commit】: Not found QC.");
            return;
        };
        let justify = commit.justify.clone().unwrap();

        if let None = justify.signature {
            eprintln!("【Commit】: Not found signature.");
            return;
        }
        let signature = justify.signature.clone().unwrap();

        // match QC
        if justify.msg_type != PRE_COMMIT || justify.view_num != self.state.view {
            eprintln!("【Commit】: QC is invalid.");
            return;
        }
        // verify signature
        let precommit_msg_hash = get_message_hash(PRE_COMMIT, justify.view_num, &justify.block);
        if !self
            .keypair
            .as_ref()
            .expect("Keypair is not found!")
            .threshold_verify(&signature, precommit_msg_hash.as_bytes())
        {
            eprintln!("【Commit】: Signature is invalid.");
            return;
        }

        // set lockedQC
        self.state.locked_qc = Some(justify.clone());

        // vote
        println!("Start commit vote!");
        let sign_msg_hash = get_message_hash(COMMIT, commit.view_num, &justify.block);
        if let Some(keypair) = &self.keypair {
            let partial_signature = keypair.sign(sign_msg_hash.as_bytes());

            let vote = Vote {
                view_num: commit.view_num,
                block: justify.block.clone(),
                partial_signature,
                from_peer_id: current_peer_id.to_vec(),
            };

            let vote_msg = Message {
                msg_type: MessageType::CommitVote(vote),
            };

            let serialized_vote_msg = coder::serialize_into_bytes(&vote_msg);
            self.send_message(&serialized_vote_msg).await;
        } else {
            eprintln!("Keypair is not found!");
            return;
        }
    }

    pub async fn handle_commit_vote(&mut self, vote: &Vote, current_peer_id: &[u8]) {
        let commit_msg_hash = get_message_hash(COMMIT, vote.view_num, &vote.block);
        let pk_info = self.get_public_key_info_by_peer_id(&vote.from_peer_id);

        // verify partial signature
        if !pk_info
            .public_key
            .verify(&vote.partial_signature, commit_msg_hash.as_bytes())
        {
            eprintln!("Partial signature is invalid!");
            return;
        }

        self.log.record_messgae_partial_signature(
            &commit_msg_hash,
            pk_info.number,
            &vote.partial_signature,
        );

        let partial_sig_count = self
            .log
            .get_partial_signatures_count_by_message_hash(&commit_msg_hash);
        let threshold = 2 * self.state.fault_tolerance_count + 1;
        if partial_sig_count != threshold as usize {
            return;
        }

        let partial_sigs = self
            .log
            .get_partial_signatures_by_message_hash(&commit_msg_hash);
        let signature = self
            .keypair
            .as_ref()
            .unwrap()
            .combine_partial_signatures(partial_sigs);

        let mut commit_qc = QC::new(COMMIT, vote.view_num, &vote.block);
        commit_qc.set_signature(&signature);

        // broadcast commit message
        let decide = Decide {
            view_num: vote.view_num,
            justify: Some(commit_qc),
        };
        let decide_msg = Message {
            msg_type: MessageType::Decide(decide),
        };
        let serialized_msg = coder::serialize_into_bytes(&decide_msg);
        self.send_message(&serialized_msg).await;

        println!("broadcast commitQC!");

        self.send_end_message(&vote.block).await;
        // Leader
        self.view_timeout_stop();
        self.newview(current_peer_id).await;
    }

    pub async fn handle_decide(&mut self, decide: &Decide, current_peer_id: &[u8]) {
        if let None = decide.justify {
            eprintln!("【Decide】: Not found QC.");
            return;
        };
        let justify = decide.justify.clone().unwrap();

        if let None = justify.signature {
            eprintln!("【Decide】: Not found signature.");
            return;
        }
        let signature = justify.signature.clone().unwrap();

        // match QC
        if justify.msg_type != COMMIT || justify.view_num != self.state.view {
            eprintln!("【Decide】: QC is invalid.");
            return;
        }
        // verify signature
        let commit_msg_hash = get_message_hash(COMMIT, justify.view_num, &justify.block);
        if !self
            .keypair
            .as_ref()
            .expect("Keypair is not found!")
            .threshold_verify(&signature, commit_msg_hash.as_bytes())
        {
            eprintln!("【Decide】: Signature is invalid.");
            return;
        }

        println!("The block command is executable:");

        println!("{:?}", justify.block);

        // stop view timeout
        self.view_timeout_stop();

        // send End message
        self.send_end_message(&justify.block).await;
        let dt = chrono::Local::now();
        let timestamp: i64 = dt.timestamp_millis();
        println!("Request:{}结束时间为：{}",justify.clone().block.cmd,timestamp);
        // newview
        self.newview(current_peer_id).await;
    }

    pub fn next_view(&mut self) {
        self.state.view = self.state.view + 1;
    }

    pub fn safe_node(&self, block: &Block, qc: &QC) -> bool {
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
