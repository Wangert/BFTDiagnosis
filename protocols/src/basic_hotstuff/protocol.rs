use components::behaviour::{PhaseState, ProtocolBehaviour, SendType};
use components::message::{Request, ConsensusData, ConsensusDataMessage};
use components::common::get_request_hash;
use crate::{
    // behaviour::ProtocolMessage::ProtocolDefault,
    basic_hotstuff::{
        common::{generate_bls_keys, get_message_hash, COMMIT, PREPARE, PRE_COMMIT},
        message::Vote,
        state::State,
        log::Log,
    },
};
use crate::{
    basic_hotstuff::message::{
        Block, Commit, ConsensusMessage, ConsensusNodePKInfo, Decide, MessageType, NewView,
        PreCommit, Prepare, QC,
    },
};
use libp2p::{gossipsub::IdentTopic, PeerId};
use network::peer::Peer;
use std::thread::sleep;
use std::time::{Instant, SystemTime};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use std::{thread, time};
use blsttc::SignatureShare;
use tokio::{
    sync::{Mutex, Notify},
    time::Sleep,
};
use utils::{coder, crypto::threshold_blsttc::TBLSKey};

use super::common::get_block_hash;

pub struct BasicHotstuffProtocol {
    pub state: State,
    pub log: Log,
    pub peer_id: String,
    pub phase_map: HashMap<u8, String>,
    pub keypair: Option<TBLSKey>,
    pub consensus_nodes: HashMap<String, PeerId>,
    pub pk_consensus_nodes: HashMap<u64, Vec<u8>>,
    pub consensus_node_pks: HashMap<Vec<u8>, ConsensusNodePKInfo>,
    pub view_timeout_notify: Arc<Notify>,
    pub view_timeout_stop_notify: Arc<Notify>,
}

impl Default for BasicHotstuffProtocol {
    fn default() -> Self {
        Self {
            state: State::new(),
            log: Log::new(),
            keypair: None,
            consensus_nodes: HashMap::new(),
            pk_consensus_nodes: HashMap::new(),
            consensus_node_pks: HashMap::new(),
            peer_id: PeerId::random().to_string(),

            view_timeout_notify: Arc::new(Notify::new()),
            view_timeout_stop_notify: Arc::new(Notify::new()),
            // new_round: false,
            phase_map: HashMap::new(),
        }
    }
}

impl BasicHotstuffProtocol {
    pub fn view_timeout_start(&self, view_num: u64) {
        println!("==============【view timeout start】==============");
        let timeout_notify = self.view_timeout_notify.clone();
        let stop_notify = self.view_timeout_stop_notify.clone();
        let current_timeout = Duration::from_secs(self.state.current_view_timeout);
        // let is_primary = self.state.is_primary.clone();
        //let mode = self.mode.clone();
        println!("Current view timeout: {}", self.state.current_view_timeout);
        tokio::spawn(async move {
            if let Err(_) = tokio::time::timeout(current_timeout, stop_notify.notified()).await {
                timeout_notify.notify_one();
                println!("View({}) is timeout!", view_num);
            }
        });
    }

    pub fn distribute_keys(&mut self, current_peer_id: PeerId) -> PhaseState {
        let sys_time1 = SystemTime::now();
        let mut send_queue = VecDeque::new();
        println!("consensus_nodes:{:?}", &self.consensus_nodes.clone());
        let distribute_tbls_key_vec =
            generate_bls_keys(&self.consensus_nodes, self.state.fault_tolerance_count);

        println!("key count: {}", distribute_tbls_key_vec.len());
        let key = distribute_tbls_key_vec[0].tbls_key.clone();
        let key_msg = ConsensusMessage {
            msg_type: MessageType::TBLSKey(key),
        };
        let serialized_key = coder::serialize_into_json_bytes(&key_msg);
        println!("{:?}", serialized_key);
        let de_key: ConsensusMessage = coder::deserialize_for_json_bytes(&serialized_key);
        println!("{:#?}", de_key);

        let mut consensus_node_pks: HashMap<Vec<u8>, ConsensusNodePKInfo> = HashMap::new();
        for key_info in distribute_tbls_key_vec {
            let msg = ConsensusMessage {
                msg_type: MessageType::TBLSKey(key_info.tbls_key.clone()),
            };

            let serialized_msg = coder::serialize_into_json_bytes(&msg);

            if key_info.peer_id.clone().to_string().as_str() == self.peer_id.as_str() {
                self.storage_tbls_key(
                    PeerId::from_str(self.peer_id.as_str()).expect("msg"),
                    &key_info.tbls_key.clone(),
                );
            }

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
        let info_data = coder::serialize_into_bytes(&consensus_node_pks.clone());
        let msg = ConsensusMessage {
            msg_type: MessageType::ConsensusNodePKsInfo(info_data),
        };
        let serialized_msg = coder::serialize_into_json_bytes(&msg);
        let data = SendType::Broadcast(serialized_msg);
        send_queue.push_back(data);
        let data = self.storage_consensus_node_pk(&consensus_node_pks, current_peer_id.clone());
        if let PhaseState::ContinueExecute(msg) = data {
            for i in msg {
                send_queue.push_back(i);
            }
        }
        let sys_time2 = SystemTime::now();

        let difference = sys_time2.duration_since(sys_time1);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        println!("Distribute key time spent: {:?}", difference);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");

        return PhaseState::ContinueExecute(send_queue);
    }



    // storage TBLS keypair
    pub fn storage_tbls_key(&mut self, current_peer_id: PeerId, tbls_key: &TBLSKey) {
        self.keypair = Some(tbls_key.clone());
    }

    pub fn storage_consensus_node_pk(
        &mut self,
        pks_info: &HashMap<Vec<u8>, ConsensusNodePKInfo>,
        current_peer_id: PeerId,
    ) -> PhaseState {
        println!("Enter storage_consensus_node_pk");
        for (peer_id, pk_info) in pks_info {
            let id = PeerId::from_bytes(peer_id).unwrap();
            println!("【【【【{}】】】】", id.to_string());
            println!("{:#?}", pk_info);

            self.pk_consensus_nodes
                .insert(pk_info.number, peer_id.clone().to_vec());
            println!("Consensus nodes count: {}", self.pk_consensus_nodes.len());
        }

        self.newview(&current_peer_id.to_bytes())
    }

    pub fn get_public_key_info_by_peer_id(&self, peer_id: &[u8]) -> &ConsensusNodePKInfo {
        let pk_info = self.consensus_node_pks.get(peer_id).expect("msg");
        pk_info
    }

    pub fn change_leader(&mut self, view_num: u64) {
        let node_count = self.pk_consensus_nodes.len();
        let leader_num = view_num % node_count as u64;

        self.state.primary = self.pk_consensus_nodes.get(&leader_num).unwrap().to_vec();
    }

    pub fn newview(&mut self, current_peer_id: &[u8]) -> PhaseState {
        let sys_time1 = SystemTime::now();

        let mut send_queue = VecDeque::new();
        // get next leader
        let next_view = self.state.view + 1;
        self.change_leader(next_view);
        println!("leader is : {:?}", self.state.primary);
        println!("I am : {:?}", current_peer_id.to_vec());

        match self.state.primary.cmp(&current_peer_id.to_vec()) {
            Ordering::Equal => {
                println!("I am the new leader!");
            }
            _ => {
                println!("I am not leader~");
                // send newview message to next leader
                let newview = NewView {
                    view_num: self.state.view,
                    justify: self.state.prepare_qc.clone(),
                    from_peer_id: current_peer_id.to_vec(),
                };

                let newview_msg = ConsensusMessage {
                    msg_type: MessageType::NewView(newview),
                };

                let serialized_msg = coder::serialize_into_json_bytes(&newview_msg);
                send_queue.push_back(SendType::Unicast(
                    PeerId::from_bytes(&self.state.primary).expect(""),
                    serialized_msg,
                ));
            }
        }

        self.state.view = next_view;

        let peer = PeerId::from_bytes(&self.state.primary).unwrap();
        println!("Current leader: {}", peer.to_string());
        println!("The final send:{:?}", send_queue.clone());
        // start new view timing
        self.view_timeout_start(next_view);

        let sys_time2 = SystemTime::now();

        let difference = sys_time2.duration_since(sys_time1);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        println!("Newview time spent: {:?}", difference);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");

        return PhaseState::ContinueExecute(send_queue);
    }

    pub fn safe_node(&self, block: &Block, qc: &QC) -> bool {
        if let Some(locked_qc) = &self.state.locked_qc {
            let locked_qc_block_hash = get_block_hash(&locked_qc.block);
            (qc.view_num > locked_qc.view_num) || block.parent_hash.eq(&locked_qc_block_hash)
        } else {
            (qc.view_num > 0) || block.parent_hash.eq("")
        }
    }

    pub fn handle_newview(&mut self, newview: &NewView, current_peer_id: &[u8]) -> PhaseState {
        let mut send_queue = VecDeque::new();
        println!("*************************** Handle newview! ************************");
        self.log.record_newview(newview.view_num, newview);

        let count = self.log.get_newviews_count_by_view(newview.view_num);
        let threshold = 2 * self.state.fault_tolerance_count + 1;
        // 2f+1 newview, calculate highQC
        if count == threshold as usize {
            println!("ready.");
            // self.new_round = true;
            let high_qc = self.log.get_high_qc_by_view(newview.view_num, self.state.prepare_qc.clone());
            self.state.high_qc = high_qc;

            return PhaseState::Over(None);
        }
        return PhaseState::ContinueExecute(send_queue);
    }

    pub fn handle_request(&mut self, request: &Request, current_peer_id: &[u8]) -> PhaseState {
        let sys_time1 = SystemTime::now();
        println!("*********************** Get the Request ***********************************");
        // verify request message signature
        let mut send_query = VecDeque::new();
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
        let msg = ConsensusMessage {
            msg_type: MessageType::Prepare(prepare),
        };
        let serialized_msg = coder::serialize_into_json_bytes(&msg);

        println!("*********************** Broadcast Prepare message *************************");

        send_query.push_back(SendType::Broadcast((serialized_msg)));

        let sys_time2 = SystemTime::now();

        let difference = sys_time2.duration_since(sys_time1);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        println!("Handle Request spent: {:?}", difference);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");

        PhaseState::ContinueExecute(send_query)
    }

    pub fn handle_prepare(&mut self, prepare: &Prepare, current_peer_id: &[u8]) -> PhaseState {
        let sys_time1 = SystemTime::now();

        let mut send_query = VecDeque::new();
        println!("*********************** Get the Prepare Message ***************************");

        // verify prepare signature
        let qc = prepare.justify.clone();
        if let None = qc {
            if prepare.block.parent_hash != "" {
                eprintln!("Current block does not extend from justify block.");
                return PhaseState::ContinueExecute(send_query);
            }
        }

        if let Some(ref qc) = qc {
            let qc_block_hash = get_block_hash(&qc.block);
            if qc_block_hash.ne(&prepare.block.parent_hash) {
                eprintln!("Current block does not extend from justify block.");
                return PhaseState::ContinueExecute(send_query);
            }

            if !self.safe_node(&prepare.block, qc) {
                eprintln!("Safenode authentication failed");
                return PhaseState::ContinueExecute(send_query);
            }
        }

        let sys_time2 = SystemTime::now();
        let difference1 = sys_time2.duration_since(sys_time1);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        println!("Prepare verify time spent: {:?}", difference1);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");

        self.state.primary = prepare.from_peer_id.clone();

        // vote
        let sign_msg_hash = get_message_hash(PREPARE, prepare.view_num, &prepare.block);
        if let Some(keypair) = &self.keypair {
            let partial_signature = keypair.sign(sign_msg_hash.as_bytes());
            let sys_time3 = SystemTime::now();
            let difference2 = sys_time3.duration_since(sys_time2);
            println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
            println!("Prepare sigh time spent: {:?}", difference2);
            println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");

            let vote = Vote {
                view_num: prepare.view_num,
                block: prepare.block.clone(),
                partial_signature: Some(partial_signature),
                from_peer_id: current_peer_id.to_vec(),
            };

            let vote_msg = ConsensusMessage {
                msg_type: MessageType::PrepareVote(vote),
            };

            let serialized_vote_msg = coder::serialize_into_json_bytes(&vote_msg);
            let leader_id = PeerId::from_bytes(&self.state.primary).expect("Leader peer id error.");

            send_query.push_back(SendType::Unicast((leader_id), (serialized_vote_msg)));
            println!(
                "*********************** Send PrepareVote message ****************************"
            );

            let sys_time4 = SystemTime::now();

            let difference = sys_time4.duration_since(sys_time3);
            println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
            println!("Handle prepare time spent: {:?}", difference);
            println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");

            return PhaseState::ContinueExecute(send_query);
        } else {
            eprintln!("Keypair is not found!");
            return PhaseState::ContinueExecute(send_query);
        }
    }

    // Leader collects the prepare vote
    pub fn handle_prepare_vote(&mut self, vote: &Vote, _current_peer_id: &[u8]) -> PhaseState {
        let mut send_query = VecDeque::new();
        println!("*********************** Leader Get the Prepare Vote *************************");
        let prepare_msg_hash = get_message_hash(PREPARE, vote.view_num, &vote.block);
        let pk_info = self.get_public_key_info_by_peer_id(&vote.from_peer_id);
        let sys_time1 = SystemTime::now();
        // verify partial signature
        if !pk_info.public_key.verify(
            &vote.partial_signature.clone().unwrap(),
            prepare_msg_hash.as_bytes(),
        ) {
            eprintln!("Partial signature is invalid!");
            return PhaseState::ContinueExecute(send_query);
        }
        let sys_time2 = SystemTime::now();

        self.log.record_messgae_partial_signature(
            &prepare_msg_hash,
            pk_info.number,
            &vote.partial_signature.clone().unwrap(),
        );

        let partial_sig_count =
            self.log.get_partial_signatures_count_by_message_hash(&prepare_msg_hash);
        let threshold = 2 * self.state.fault_tolerance_count + 1;
        
        let difference3 = sys_time2.duration_since(sys_time1);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        println!("init time spent: {:?}", difference3);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        if partial_sig_count != threshold as usize {
            return PhaseState::ContinueExecute(send_query);
        }

        println!("Leader has collected 2 * f + 1 Vote!");
        
        let partial_sigs = self.log.get_partial_signatures_by_message_hash(&prepare_msg_hash);
        
        let signature = self
            .keypair
            .as_ref()
            .unwrap()
            .combine_partial_signatures(partial_sigs);
        let sys_time3 = SystemTime::now();
        let difference = sys_time3.duration_since(sys_time2);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        println!("Combine time spent: {:?}", difference);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");

        let mut prepare_qc = QC::new(PREPARE, vote.view_num, &vote.block);
        prepare_qc.set_signature(&signature);

        // broadcast pre-commit message
        let pre_commit = PreCommit {
            view_num: vote.view_num,
            justify: Some(prepare_qc),
        };
        let pre_commit_msg = ConsensusMessage {
            msg_type: MessageType::PreCommit(pre_commit),
        };
        let serialized_msg = coder::serialize_into_json_bytes(&pre_commit_msg);

        println!("*********************** Broadcast PreCommit message ***********************");
        send_query.push_back(SendType::Broadcast(serialized_msg));

        let sys_time4 = SystemTime::now();

        let difference = sys_time4.duration_since(sys_time3);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        println!("Vote Prepare time spent: {:?}", difference);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");

        return PhaseState::ContinueExecute(send_query);
    }

    pub fn handle_precommit(
        &mut self,
        pre_commit: &PreCommit,
        current_peer_id: &[u8],
    ) -> PhaseState {
        let sys_time1 = SystemTime::now();
        let mut send_query = VecDeque::new();
        println!("*********************** Get the PreCommit *********************************");
        if let None = pre_commit.justify {
            eprintln!("【PreCommit】: Not found QC.");
            return PhaseState::ContinueExecute(send_query);
        };
        let justify = pre_commit.justify.clone().unwrap();

        if let None = justify.signature {
            eprintln!("【PreCommit】: Not found signature.");
            return PhaseState::ContinueExecute(send_query);
        }
        let signature = justify.signature.clone().unwrap();

        // match QC
        if justify.msg_type != PREPARE || justify.view_num != self.state.view {
            eprintln!("【PreCommit】: QC is invalid.");
            return PhaseState::ContinueExecute(send_query);
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
            return PhaseState::ContinueExecute(send_query);
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
                partial_signature: Some(partial_signature),
                from_peer_id: current_peer_id.to_vec(),
            };

            let vote_msg = ConsensusMessage {
                msg_type: MessageType::PreCommitVote(vote),
            };

            let serialized_vote_msg = coder::serialize_into_json_bytes(&vote_msg);
            let leader_id = PeerId::from_bytes(&self.state.primary).expect("Leader peer id error.");
            println!("*********************** Send PreCommitVote message ************************");

            send_query.push_back(SendType::Unicast(leader_id, serialized_vote_msg));

            let sys_time2 = SystemTime::now();

            let difference = sys_time2.duration_since(sys_time1);
            println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
            println!("Handle Precommit time spent: {:?}", difference);
            println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");

            return PhaseState::ContinueExecute(send_query);
        } else {
            eprintln!("Keypair is not found!");
            return PhaseState::ContinueExecute(VecDeque::new());
        }
    }

    // Leader collects the precommit vote
    pub fn handle_precommit_vote(&mut self, vote: &Vote, _current_peer_id: &[u8]) -> PhaseState {
        let sys_time1 = SystemTime::now();
        let mut send_query = VecDeque::new();

        println!("*********************** Leader Get the PreCommit Vote ***********************");
        let precommit_msg_hash = get_message_hash(PRE_COMMIT, vote.view_num, &vote.block);
        let pk_info = self.get_public_key_info_by_peer_id(&vote.from_peer_id);

        // verify partial signature
        if !pk_info.public_key.verify(
            &vote.partial_signature.clone().unwrap(),
            precommit_msg_hash.as_bytes(),
        ) {
            eprintln!("Partial signature is invalid!");
            return PhaseState::ContinueExecute(send_query);
        }

        self.log.record_messgae_partial_signature(
            &precommit_msg_hash,
            pk_info.number,
            &vote.partial_signature.clone().unwrap(),
        );

        let partial_sig_count =
            self.log.get_partial_signatures_count_by_message_hash(&precommit_msg_hash);
        let threshold = 2 * self.state.fault_tolerance_count + 1;
        if partial_sig_count != threshold as usize {
            return PhaseState::ContinueExecute(send_query);
        }

        println!("Leader has collected 2 * f + 1 Vote!");

        let partial_sigs = self.log.get_partial_signatures_by_message_hash(&precommit_msg_hash);
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
        let commit_msg = ConsensusMessage {
            msg_type: MessageType::Commit(commit),
        };
        let serialized_msg = coder::serialize_into_json_bytes(&commit_msg);

        println!("*********************** Broadcast Commit message **************************");

        send_query.push_back(SendType::Broadcast(serialized_msg));

        let sys_time2 = SystemTime::now();

        let difference = sys_time2.duration_since(sys_time1);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        println!("Vote Precommit time spent: {:?}", difference);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");

        return PhaseState::ContinueExecute(send_query);
    }

    pub fn handle_commit(&mut self, commit: &Commit, current_peer_id: &[u8]) -> PhaseState {
        let sys_time1 = SystemTime::now();
        println!("*********************** Get the Commit ************************************");
        let mut send_query = VecDeque::new();
        if let None = commit.justify {
            eprintln!("【Commit】: Not found QC.");
            return PhaseState::ContinueExecute(send_query);
        };
        let justify = commit.justify.clone().unwrap();

        if let None = justify.signature {
            eprintln!("【Commit】: Not found signature.");
            return PhaseState::ContinueExecute(send_query);
        }
        let signature = justify.signature.clone().unwrap();

        // match QC
        if justify.msg_type != PRE_COMMIT || justify.view_num != self.state.view {
            eprintln!("【Commit】: QC is invalid.");
            return PhaseState::ContinueExecute(send_query);
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
            return PhaseState::ContinueExecute(send_query);
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
                partial_signature: Some(partial_signature),
                from_peer_id: current_peer_id.to_vec(),
            };

            let vote_msg = ConsensusMessage {
                msg_type: MessageType::CommitVote(vote),
            };

            let serialized_vote_msg = coder::serialize_into_json_bytes(&vote_msg);
            let leader_id = PeerId::from_bytes(&self.state.primary).expect("Leader peer id error.");
            println!("*********************** Send CommitVote message ***************************");

            send_query.push_back(SendType::Unicast(leader_id, serialized_vote_msg));

            let sys_time2 = SystemTime::now();

            let difference = sys_time2.duration_since(sys_time1);
            println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
            println!("Handle Commit time spent: {:?}", difference);
            println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");

            return PhaseState::ContinueExecute(send_query);
        } else {
            eprintln!("Keypair is not found!");
            return PhaseState::ContinueExecute(send_query);
        }
    }

    pub fn handle_commit_vote(&mut self, vote: &Vote, current_peer_id: &[u8]) -> PhaseState {
        let sys_time1 = SystemTime::now();
        let mut send_query = VecDeque::new();
        println!("*********************** Leader Get the Commit Vote **************************");
        let commit_msg_hash = get_message_hash(COMMIT, vote.view_num, &vote.block);
        let pk_info = self.get_public_key_info_by_peer_id(&vote.from_peer_id);

        // verify partial signature
        if !pk_info.public_key.verify(
            &vote.partial_signature.clone().unwrap(),
            commit_msg_hash.as_bytes(),
        ) {
            eprintln!("Partial signature is invalid!");
            return PhaseState::ContinueExecute(send_query);
        }

        self.log.record_messgae_partial_signature(
            &commit_msg_hash,
            pk_info.number,
            &vote.partial_signature.clone().unwrap(),
        );

        let partial_sig_count = self.log.get_partial_signatures_count_by_message_hash(&commit_msg_hash);
        let threshold = 2 * self.state.fault_tolerance_count + 1;
        if partial_sig_count != threshold as usize {
            return PhaseState::ContinueExecute(send_query);
        }

        println!("Leader has collected 2 * f + 1 Vote!");

        let partial_sigs = self.log.get_partial_signatures_by_message_hash(&commit_msg_hash);
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
            justify: Some(commit_qc.clone()),
        };
        let decide_msg = ConsensusMessage {
            msg_type: MessageType::Decide(decide),
        };
        let serialized_msg = coder::serialize_into_json_bytes(&decide_msg);

        println!("*********************** Broadcast Decide message **************************");

        send_query.push_back(SendType::Broadcast(serialized_msg));

        let request = Request {
            cmd: commit_qc.block.cmd.clone(),
            // timestamp: todo!(),
        };

        let message = ConsensusMessage {
            msg_type: MessageType::Request(request.clone()),
        };
        let data = coder::serialize_into_json_bytes(&message);

        println!("***************************Start new view *********************************");

        // Leader
        self.view_timeout_stop();
        let messages = self.newview(current_peer_id);
        if let PhaseState::ContinueExecute(msg) = messages {
            for message in msg {
                send_query.push_back(message);
            }
        };

        let sys_time2 = SystemTime::now();

        let difference = sys_time2.duration_since(sys_time1);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        println!("Vote Commit time spent: {:?}", difference);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");

        return PhaseState::Complete(request, send_query);
    }

    pub fn handle_decide(&mut self, decide: &Decide, current_peer_id: &[u8]) -> PhaseState {
        let sys_time1 = SystemTime::now();
        let mut send_query = VecDeque::new();
        println!("*********************** Get the Decide ************************************");
        let decide_clone = decide.clone();
        if let None = decide.justify {
            eprintln!("【Decide】: Not found QC.");
            return PhaseState::ContinueExecute(send_query);
        };
        let justify = decide.justify.clone().unwrap();

        if let None = justify.signature {
            eprintln!("【Decide】: Not found signature.");
            return PhaseState::ContinueExecute(send_query);
        }
        let signature = justify.signature.clone().unwrap();

        // match QC
        if justify.msg_type != COMMIT || justify.view_num != self.state.view {
            eprintln!("【Decide】: QC is invalid.");
            return PhaseState::ContinueExecute(send_query);
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
            return PhaseState::ContinueExecute(send_query);
        }

        println!("The block command is executable:");
        println!("{:?}", justify.block);

        let request = Request {
            cmd: decide_clone.justify.unwrap().block.cmd,
        };
        let message = ConsensusMessage {
            msg_type: MessageType::Request(request.clone()),
        };
        let data = coder::serialize_into_json_bytes(&message);

        // stop view timeout
        self.view_timeout_stop();

        println!("***************************Start new view *********************************");

        // newview
        let messages = self.newview(current_peer_id);
        if let PhaseState::ContinueExecute(msg) = messages {
            for message in msg {
                send_query.push_back(message);
            }
        };

        let sys_time2 = SystemTime::now();

        let difference = sys_time2.duration_since(sys_time1);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        println!("Handle Decide time spent: {:?}", difference);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");

        return PhaseState::Complete(request, send_query);
    }

    pub fn next_view(&mut self) {
        self.state.view = self.state.view + 1;
    }

    pub fn view_timeout_stop(&mut self) {
        println!("timeout stop");
        self.state.current_view_timeout = self.state.tf;
        self.view_timeout_stop_notify.notify_one();
    }
}

impl ProtocolBehaviour for BasicHotstuffProtocol {
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
        self.state.current_id = current_peer_id.clone();
        self.peer_id = PeerId::from_bytes(&current_peer_id.clone())
            .expect("Found invalid UTF-8")
            .to_string();

        let data =
            self.distribute_keys(PeerId::from_bytes(&current_peer_id.clone()).expect("Error"));
        return data;
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

        let message: ConsensusMessage = coder::deserialize_for_json_bytes(_msg);
        match message.msg_type {
            MessageType::Request(msg) => {
                self.state.current_request = msg.clone();
                return self.handle_request(&msg, &current_peer_id);
            }
            MessageType::NewView(msg) => {
                return self.handle_newview(&msg, &current_peer_id);
            }
            MessageType::Prepare(msg) => {
                return self.handle_prepare(&msg, &current_peer_id);
            }
            MessageType::Commit(msg) => {
                return self.handle_commit(&msg, &current_peer_id);
            }
            MessageType::PreCommit(msg) => {
                return self.handle_precommit(&msg, &current_peer_id);
            }
            MessageType::Decide(msg) => {
                return self.handle_decide(&msg, &current_peer_id);
            }
            MessageType::PrepareVote(msg) => {
                return self.handle_prepare_vote(&msg, &current_peer_id);
            }
            MessageType::CommitVote(msg) => {
                return self.handle_commit_vote(&msg, &current_peer_id);
            }
            MessageType::PreCommitVote(msg) => {
                return self.handle_precommit_vote(&msg, &current_peer_id);
            }
            MessageType::End(msg) => {}
            MessageType::TBLSKey(msg) => {
                println!("Received TBLSKey");
                self.storage_tbls_key(PeerId::from_bytes(&current_peer_id).expect("msg"), &msg);
            }
            MessageType::ConsensusNodePKsInfo(msg) => {
                println!("Received ConsensusNodePKsInfo!");
                let info: HashMap<Vec<u8>, ConsensusNodePKInfo> =
                    coder::deserialize_for_bytes(&msg);
                self.consensus_node_pks = info.clone();

                return self.storage_consensus_node_pk(
                    &info,
                    PeerId::from_bytes(&current_peer_id).expect("msg"),
                );
            }
        }
        let mut queue = VecDeque::new();
        queue.push_back(SendType::Broadcast(vec![]));
        PhaseState::ContinueExecute(queue)
    }

    fn receive_consensus_requests(&mut self, requests: Vec<Request>) {
        todo!()
    }

    fn get_current_phase(&mut self, _msg: &[u8]) -> u8 {
        if _msg.len() == 0 {
            return 0;
        }
        let data: ConsensusMessage = coder::deserialize_for_json_bytes(_msg);
        let i = match data.msg_type {
            MessageType::Request(_) => 0,
            MessageType::NewView(_) => 0,
            MessageType::Prepare(_) => 1,
            MessageType::Commit(_) => 3,
            MessageType::PreCommit(_) => 2,
            MessageType::Decide(_) => 0,
            MessageType::PrepareVote(_) => 1,
            MessageType::CommitVote(_) => 3,
            MessageType::PreCommitVote(_) => 2,
            MessageType::End(_) => 0,
            MessageType::TBLSKey(_) => 0,
            MessageType::ConsensusNodePKsInfo(_) => 0,
        };
        return i;
    }

    fn protocol_phases(&mut self) -> HashMap<u8, Vec<u8>> {
        let mut hash_map = HashMap::new();
        println!("No security test interface is implemented！");
        let block = Block {
            cmd: "test".to_string(),
            parent_hash: "test".to_string(),
        };

        let qc = QC {
            msg_type: 1,
            view_num: 1,
            block: block.clone(),
            signature: None,
        };
        let prepare = Prepare {
            view_num: 1,
            block: block.clone(),
            justify: Some(qc.clone()),
            from_peer_id: vec![1],
        };
        let vote = Vote {
            view_num: 1,
            block: block.clone(),
            partial_signature: None,
            from_peer_id: vec![1],
        };
        let precommit = PreCommit {
            view_num: 1,
            justify: Some(qc.clone()),
        };
        let commit = Commit {
            view_num: 1,
            justify: Some(qc.clone()),
        };

        let prepare_json_bytes = coder::serialize_into_json_str(&prepare).as_bytes().to_vec();
        let vote = coder::serialize_into_json_str(&vote).as_bytes().to_vec();
        let pre_commit_json_bytes = coder::serialize_into_json_str(&precommit)
            .as_bytes()
            .to_vec();
        let commit_json_bytes = coder::serialize_into_json_str(&commit).as_bytes().to_vec();

        hash_map.insert(1, prepare_json_bytes);
        hash_map.insert(2, vote.clone());
        hash_map.insert(3, pre_commit_json_bytes);
        hash_map.insert(4, vote.clone());
        hash_map.insert(5, commit_json_bytes);
        hash_map.insert(6, vote.clone());

        self.phase_map.insert(1, String::from("Prepare"));
        self.phase_map.insert(2, String::from("Vote"));
        self.phase_map.insert(3, String::from("PreCommit"));
        self.phase_map.insert(4, String::from("Vote"));
        self.phase_map.insert(5, String::from("Commit"));
        self.phase_map.insert(6, String::from("Vote"));
        hash_map
    }

    fn phase_map(&self) -> HashMap<u8, String> {
        self.phase_map.clone()
    }

    fn is_leader(&self, peer_id: Vec<u8>) -> bool {
        match self.state.primary.cmp(&peer_id) {
            Ordering::Equal => true,
            _ => false,
        }
    }

    fn generate_serialized_request_message(&self, request: &Request) -> Vec<u8> {
        let msg = ConsensusMessage {
            msg_type: MessageType::Request(request.to_owned()),
        };
        let data = coder::serialize_into_json_bytes(&msg);
        data
    }

    fn view_timeout_handler(&mut self, current_peer_id: PeerId) -> PhaseState {
        println!("OK?");
        let current_view_timeout = self.state.current_view_timeout;
        self.state.current_view_timeout = current_view_timeout * 2;
        // send newview
        let msg = self.newview(&current_peer_id.to_bytes());
        return msg;
    }

    fn current_request(&self) -> Request {
        self.state.current_request.clone()
    }

    fn protocol_reset(&mut self) {
        self.view_timeout_stop();
        self.state.current_id = Vec::new();
        self.state.current_request = Request {
            cmd: "None".to_string(),
        };
        self.state.view = 0;
        self.state.primary = vec![];
        self.state.node_count = 4;
        self.state.fault_tolerance_count = 1;
        self.state.high_qc = None;
        self.state.prepare_qc = None;
        self.state.locked_qc = None;
        self.state.commit_qc = None;
        self.state.tf = 10;
        self.state.current_view_timeout = 10;
        self.keypair = None;
        self.consensus_nodes = HashMap::new();
        self.pk_consensus_nodes = HashMap::new();

        self.log.message_signatures = HashMap::new();
        self.log.newviews = HashMap::new();
        self.consensus_node_pks = HashMap::new();
        self.peer_id = PeerId::random().to_string();

        self.view_timeout_notify = Arc::new(Notify::new());
        self.view_timeout_stop_notify = Arc::new(Notify::new());
    }

    
}
