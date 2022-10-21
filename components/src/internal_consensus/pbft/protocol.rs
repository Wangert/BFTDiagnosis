use crate::internal_consensus::{pbft::common::{get_message_key, STABLE_CHECKPOINT_DELTA}};
use libp2p::PeerId;
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    time::Duration, str::FromStr,
};
use super::{log::{ConsensusLog,LogPhaseState}, state::State};
use storage::database::LevelDB;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Notify,
};
use utils::{
    coder::{self, get_hash_str},
    crypto::eddsa::{EdDSAKeyPair, EdDSAPublicKey},
};

use crate::behaviour::{PhaseState, ProtocolBehaviour, SendType};
use crate::internal_consensus::pbft::message::ConsensusMessage;

use crate::internal_consensus::pbft::message::{
    CheckPoint, Commit, MetaReply, NewView, PrePrepare, Prepare, ProofMessages, PublicKey, Reply, ViewChange,
};
use crate::message::Request;
use crate::internal_consensus::pbft::message::MessageType;

pub struct PBFTProtocol {
    pub state: State,
    pub log: Box<ConsensusLog>,
    pub taken_requests: HashSet<Vec<u8>>,
    // pub db: Box<LevelDB>,
    pub keypair: Box<EdDSAKeyPair>,
    pub viewchange_notify: Arc<Notify>,
    
    pub timeout_notify: Arc<Notify>,

    pub consensus_node_pk: HashMap<Vec<u8>, Vec<u8>>,

    pub view_timeout_notify: Arc<Notify>,
    pub view_timeout_stop_notify: Arc<Notify>,
    pub current_view_timeout: u64,
}

impl Default for PBFTProtocol {
    fn default() -> Self {
        Self {
            state: State::new(Duration::from_secs(5)),
            log: Box::new(ConsensusLog::new()),
            // db: Box::new(LevelDB::new(db_path)),
            keypair: Box::new(EdDSAKeyPair::new()),
            
            viewchange_notify: Arc::new(Notify::new()),
            timeout_notify: Arc::new(Notify::new()),
            consensus_node_pk: HashMap::new(),
            taken_requests: HashSet::new(),

            view_timeout_notify: Arc::new(Notify::new()),
            view_timeout_stop_notify: Arc::new(Notify::new()),
            current_view_timeout: 10,
        }
    }
}

impl PBFTProtocol {

    pub fn view_timeout_start(&self, view_num: u64) {
        println!("==============【view timeout start】==============");
        let timeout_notify = self.view_timeout_notify.clone();
        let stop_notify = self.view_timeout_stop_notify.clone();
        let current_timeout = Duration::from_secs(self.current_view_timeout);
        // let is_primary = self.state.is_primary.clone();
        //let mode = self.mode.clone();
        println!("Current view timeout: {}", self.current_view_timeout);
        tokio::spawn(async move {
            if let Err(_) = tokio::time::timeout(current_timeout, stop_notify.notified()).await {
                timeout_notify.notify_one();
                println!("View({}) is timeout!", view_num);
            }
        });
    }

    pub fn get_public_key_by_peer_id(&self, peer_id: &[u8]) -> Option<EdDSAPublicKey> {
        let value = self.consensus_node_pk.get(peer_id);
        if let Some(pk_vec) = value {
            Some(EdDSAPublicKey(pk_vec.to_owned()))
        } else {
            None
        }
    }

    // Create a viewchange message based on the local storage messages
    // And broadcast it to other nodes to enter the viewchange mode
    pub fn broadcast_viewchange(&self, current_peer_id: &[u8]) -> PhaseState {
        let mut send_query = VecDeque::new();
        // let mode = self.state.mode.clone();
        // Avoid broadcast viewchange messages again
        // if let Mode::Abnormal = *mode.lock().await {
        //     return;
        // }
        // Switch to viewchange mode
        // *mode.lock().await = Mode::Abnormal;
        // let m = mode.lock().await.clone();
        // println!("&&&&&&&&&&&&&&{:?}&&&&&&&&&&&&&&&", m);

        let threshold = 2 * self.state.fault_tolerance_count;
        let sequence_number = self.state.stable_checkpoint.0;
        let latest_stable_checkpoint_messages = *self
            .log
            .get_checkpoint_messages_by_sequence_number(sequence_number, threshold)
            .clone();
        let proof_messages = *self.log.get_proof_messages().clone();
        let viewchange = ViewChange {
            new_view: self.state.view + 1,
            // latest_stable_checkpoint: self.state.stable_checkpoint.0,
            // latest_stable_checkpoint_messages,
            proof_messages,
            from_peer_id: current_peer_id.to_vec(),
            signature: String::from(""),
        };
        let viewchange_msg = ConsensusMessage {
            msg_type: MessageType::ViewChange(viewchange),
        };
        let serialized_viewchange_msg = coder::serialize_into_bytes(&viewchange_msg);
        
        send_query.push_back(SendType::Broadcast(serialized_viewchange_msg));
        PhaseState::ContinueExecute(send_query)
    }

    fn handle_request(&mut self, current_peer_id: &[u8], request: &Request) -> PhaseState {
        self.taken_requests.insert(coder::serialize_into_bytes(request));
        let mut send_query = VecDeque::new();
        self.state.primary = current_peer_id.to_vec();

        println!("******************* Handle request *******************");

        // create PrePrepare message
        let view = self.state.view;
        let seq_number = self.state.current_sequence_number + 1;

        let serialized_request = coder::serialize_into_bytes(&request);
        self.log.record_request(seq_number, &request.clone());
        let request = self.log.get_local_request_by_sequence_number(seq_number);
        println!("################# Current Request Messages #################");
        println!("{:?}", &request);
        println!("###############################################################");

        let m_hash = get_hash_str(&serialized_request);

        let mut preprepare = PrePrepare {
            view,
            number: seq_number,
            m_hash,
            m: serialized_request,
            signature: vec![],
            from_peer_id: current_peer_id.to_vec(),
        };

        // signature
        let preprepare_key = get_message_key(&MessageType::PrePrepare(preprepare.clone()));
        let signature = self.keypair.sign(preprepare_key.as_bytes());
        preprepare.signature = signature;

        let broadcast_msg = ConsensusMessage {
            msg_type: MessageType::PrePrepare(preprepare),
        };

        let serialized_msg = coder::serialize_into_bytes(&broadcast_msg);
        //let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

        send_query.push_back(SendType::Broadcast(serialized_msg));
        PhaseState::ContinueExecute(send_query)
    }
    fn handle_preprepare(
        &mut self,
        current_peer_id: &[u8],
        msg: &PrePrepare,
    ) -> PhaseState {
        let request:Request = coder::deserialize_for_bytes(&msg.m);
        self.taken_requests.insert(coder::serialize_into_bytes(&request));
        let mut send_query = VecDeque::new();

        self.state.primary = msg.clone().from_peer_id;

        println!("*******************Handle Preprepare*******************");


        let key_str = get_message_key(&MessageType::PrePrepare(msg.clone()));
        let source_pk = self.get_public_key_by_peer_id(&msg.from_peer_id);
        //let source_pk = self.consensus_node_pk.get(&msg.from_peer_id);
        let peer = PeerId::from_bytes(&msg.from_peer_id).expect("peer bytes error.");
        if let Some(pk) = source_pk {
            if pk.verify(&msg.signature, key_str.as_bytes()) {
                println!("PREPREPARE: {}' signature is ok", &peer.to_string());
            } else {
                eprintln!("PREPREPARE: {}' signature is error", &peer.to_string());
                return PhaseState::ContinueExecute(send_query);
            }
        } else {
            eprintln!(
                "PREPREPARE: {}' public key is not found.",
                &peer.to_string()
            );
            return PhaseState::ContinueExecute(send_query);
        }

        // check sequence number
        let low = self.state.stable_checkpoint.0;
        let high = self.state.stable_checkpoint.0 + 100;
        if msg.number <= low || msg.number > high {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return PhaseState::ContinueExecute(send_query);
        }

        // check view
        if self.state.view != msg.view {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return PhaseState::ContinueExecute(send_query);
        }

        // record request message
        let serialized_request = msg.m.clone();
        let m: Request = coder::deserialize_for_bytes(&serialized_request[..]);
        self.log.record_request(msg.number, &m);
        let request = self.log.get_local_request_by_sequence_number(msg.number);

        println!("###################Current Request Messages#################");
        println!("{:?}", &request);
        println!("###############################################################");

        // record preprepare message
        self.log
            .record_message_handler(MessageType::PrePrepare(msg.clone()));

        let msg_vec = self.log.get_local_messages_by_hash(&key_str);
        println!("###################Current PrePrepare Messages#################");
        println!("{:?}", &msg_vec);
        println!("###############################################################");

        // create prepare message
        let view = self.state.view;
        let seq_number = msg.number;
        let m_hash = msg.m_hash.clone();

        let mut prepare = Prepare {
            view,
            number: seq_number,
            m_hash,
            from_peer_id: current_peer_id.to_vec(),
            signature: vec![],
        };

        // signature
        let prepare_key = get_message_key(&MessageType::Prepare(prepare.clone()));
        let signature = self.keypair.sign(prepare_key.as_bytes());
        prepare.signature = signature;

        let broadcast_msg = ConsensusMessage {
            msg_type: MessageType::Prepare(prepare),
        };

        let serialized_msg = coder::serialize_into_bytes(&broadcast_msg);
        //let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

        // broadcast prepare message
        
        send_query.push_back(SendType::Broadcast(serialized_msg));
        //record current request consensus start time
        self.log.record_current_request(&msg.m_hash);

        PhaseState::ContinueExecute(send_query)
    }

    fn handle_prepare(&mut self, current_peer_id: &[u8], msg: &Prepare) -> PhaseState {
        println!("*******************Handle Prepare*******************");
        
        let mut send_query = VecDeque::new();

        // verify signature
        let key_str = get_message_key(&MessageType::Prepare(msg.clone()));
        let source_pk = self.get_public_key_by_peer_id(&msg.from_peer_id);
        let peer = PeerId::from_bytes(&msg.from_peer_id).expect("peer bytes error.");
        if let Some(pk) = source_pk {
            if pk.verify(&msg.signature, key_str.as_bytes()) {
                println!("PREPARE: {}' signature is ok", &peer.to_string());
            } else {
                eprintln!("PREPARE: {}' signature is error", &peer.to_string());
                return PhaseState::ContinueExecute(send_query);
            }
        } else {
            println!("PREPARE: {}' public key is not found.", &peer.to_string());
            return PhaseState::ContinueExecute(send_query);
        }


        //check current request prepared state
        let phase_state = self.log.get_request_phase_state(&msg.m_hash);
        if let Some(&LogPhaseState::Prepared) = phase_state {
            return PhaseState::ContinueExecute(send_query);
        }

        // verify prepare message
        //check sequence number
        let low = self.state.stable_checkpoint.0;
        let high = self.state.stable_checkpoint.0 + 100;
        if msg.number <= low || msg.number > high {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return PhaseState::ContinueExecute(send_query);
        }
        // check view
        if self.state.view != msg.view {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return PhaseState::ContinueExecute(send_query);
        }

        //check m hash
        let m = self.log.get_local_request_by_sequence_number(msg.number);
        if let Some(m) = m {
            let request_hash = get_message_key(&MessageType::Request(m.clone()));
            if request_hash.ne(&msg.m_hash) {
                eprintln!("Request hash error!");
                return PhaseState::ContinueExecute(send_query);
            }
        } else {
            eprintln!("Request is not exsit!");
            return PhaseState::ContinueExecute(send_query);
        }

        //record message
        self.log
            .record_message_handler(MessageType::Prepare(msg.clone()));

        let msg_vec = self.log.get_local_messages_by_hash(&key_str);
        println!("###################Current Prepare Messages#################");
        println!("{:?}", &msg_vec);
        println!("###############################################################");

        // check 2f+1 prepare messages (include current node)
        let current_count = self.log.get_local_messages_count_by_hash(&key_str);
        let threshold = 2 * self.state.fault_tolerance_count;
        
        if current_count as u64 == threshold {
            self.log
                .update_request_phase_state(&msg.m_hash, LogPhaseState::Prepared);
            //self.state.phase_state = PhaseState::Prepared;
            println!("【Prepare message to 2f+1, send commit message】");
            // create commit message
            let view = self.state.view;
            let seq_number = msg.number;
            let m_hash = msg.m_hash.clone();
            let mut commit = Commit {
                view,
                number: seq_number,
                m_hash,
                from_peer_id: current_peer_id.to_vec(),
                signature: vec![],
            };

            // signature
            let commit_key = get_message_key(&MessageType::Commit(commit.clone()));
            let signature = self.keypair.sign(commit_key.as_bytes());
            commit.signature = signature;

            let broadcast_msg = ConsensusMessage {
                msg_type: MessageType::Commit(commit),
            };

            let serialized_msg = coder::serialize_into_bytes(&broadcast_msg);
            //let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

            // broadcast commit message
            
            send_query.push_back(SendType::Broadcast(serialized_msg));
        }
        PhaseState::ContinueExecute(send_query)
    }

    fn handle_commit(&mut self, current_peer_id: &[u8], msg: &Commit) -> PhaseState {
        let mut send_query = VecDeque::new();
        let key_str = get_message_key(&MessageType::Commit(msg.clone()));
        let count = self.log.get_local_messages_count_by_hash(&key_str);
        let threshold = 2 * self.state.fault_tolerance_count;


        if count as u64 >= threshold {
            return PhaseState::ContinueExecute(send_query);
        }
        println!("*******************Handle Commit *******************");

        // verify signature
        
        let source_pk = self.get_public_key_by_peer_id(&msg.from_peer_id);
        let peer = PeerId::from_bytes(&msg.from_peer_id).expect("peer bytes error.");
        if let Some(pk) = source_pk {
            if pk.verify(&msg.signature, key_str.as_bytes()) {
                println!("COMMIT: {}' signature is ok", &peer.to_string());
            } else {
                eprintln!("COMMIT: {}' signature is error", &peer.to_string());
                return PhaseState::ContinueExecute(send_query);
            }
        } else {
            eprintln!("COMMIT: {}' public key is not found.", &peer.to_string());
            return PhaseState::ContinueExecute(send_query);
        }

        // check current request commited state
        let phase_state = self.log.get_request_phase_state(&msg.m_hash);
        if let Some(&LogPhaseState::Commited) = phase_state {
            return PhaseState::ContinueExecute(send_query);
        }

        // verify commit message
        // check sequence number
        let low = self.state.stable_checkpoint.0;
        let high = self.state.stable_checkpoint.0 + 100;
        if msg.number <= low || msg.number > high {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return PhaseState::ContinueExecute(send_query);
        }
        // check view
        if self.state.view != msg.view {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return PhaseState::ContinueExecute(send_query);
        }

        // check m hash
        let m = self.log.get_local_request_by_sequence_number(msg.number);
        let m = if let Some(m) = m {
            m.clone()
        } else {
            eprintln!("Not have local request record!");
            return PhaseState::ContinueExecute(send_query);
        };

        // check m hash
        let request_hash = get_message_key(&MessageType::Request(m.clone()));
        if request_hash.ne(&msg.m_hash) {
            eprintln!("Request hash error!");
            return PhaseState::ContinueExecute(send_query);
        }

        // record message
        self.log
            .record_message_handler(MessageType::Commit(msg.clone()));

        let key_str = get_message_key(&MessageType::Commit(msg.clone()));
        let msg_vec = self.log.get_local_messages_by_hash(&key_str);
        println!("###################Current Commit Messages#################");
        println!("{:?}", &msg_vec);
        println!("###############################################################");

        // check 2f+1 commit
        let current_count = self.log.get_local_messages_count_by_hash(&key_str);
        let threshold = 2 * self.state.fault_tolerance_count;
        if current_count as u64 > threshold {
            return PhaseState::ContinueExecute(send_query);
        }

        if current_count as u64 == threshold {
            // update checkpoint relate information
            self.state.update_checkpoint_state(&msg.m_hash);
            self.log.remove_commited_request();
            self.state.current_commited_request_count += 1;
            let current_commited_request_count =
                self.state.current_commited_request_count;
            

            self.log
                .update_request_phase_state(&msg.m_hash, LogPhaseState::Commited);
            //self.state.phase_state = PhaseState::Commited;
            println!("【Commit message to 2f+1, send reply message】");
            // create reply message
            let cmd = m.cmd;
            let view = self.state.view;
            let seq_number = msg.number;
            // let client_id = m.client_id;
            // let timestamp = m.timestamp;
            // let timestamp_clone = timestamp.clone();
            let mut reply = Reply {
                // client_id,
                // timestamp: timestamp.clone(),
                number: seq_number,
                from_peer_id: current_peer_id.to_vec(),
                signature: vec![],
                result: "ok!".as_bytes().to_vec(),
                view,
                cmd:cmd.clone(),
            };

            // signature
            let reply_key = get_message_key(&MessageType::Reply(reply.clone()));
            let signature = self.keypair.sign(reply_key.as_bytes());
            reply.signature = signature;

            let broadcast_msg = ConsensusMessage {
                msg_type: MessageType::Reply(reply),
            };
            let serialized_msg = coder::serialize_into_bytes(&broadcast_msg);
            // Send Reply message to client
            
            send_query.push_back(SendType::Unicast(PeerId::from_bytes(&self.state.primary).expect("msg"), serialized_msg));

            let request = Request {
                cmd:cmd.clone(),
                // timestamp:timestamp.clone(),
            };
            let msg = ConsensusMessage {
                msg_type: MessageType::Request(request.clone()),
            };
            let message = coder::serialize_into_bytes(&msg);


            
            self.log.reset();
            return PhaseState::Complete(request, send_query);
            
        }
        PhaseState::ContinueExecute(send_query)
    }

    fn handle_reply(&mut self, _current_peer_id: &[u8], msg: &Reply) -> PhaseState {
        let mut send_query = VecDeque::new();
        let key_str = get_message_key(&&MessageType::Reply(msg.clone()));
        let threshold = (self.state.node_count as u64 - self.state.fault_tolerance_count) as u64;

        // if count as u64 >= threshold {
        //     return PhaseState::ContinueExecute(send_query);
        // }

        println!("******************* Handle Reply *******************");

        // if let ClientState::Replied = self.state.client_state {
        //     return;
        // }
        // verify signature
        let key_str = get_message_key(&MessageType::Reply(msg.clone()));
        let source_pk = self.get_public_key_by_peer_id(&msg.from_peer_id);
        let peer = PeerId::from_bytes(&msg.from_peer_id).expect("peer bytes error.");
        if let Some(pk) = source_pk {
            if pk.verify(&msg.signature, key_str.as_bytes()) {
                println!("REPLY: {}' signature is ok", &peer.to_string());
            } else {
                eprintln!("REPLY: {}' signature is error", &peer.to_string());
                return PhaseState::ContinueExecute(send_query);
            }
        } else {
            eprintln!("REPLY: {}' public key is not found.", &peer.to_string());
            return PhaseState::ContinueExecute(send_query);
        }

        // record reply
        self.log
            .record_message_handler(MessageType::Reply(msg.clone()));
        let key_str = get_message_key(&MessageType::Reply(msg.clone()));
        let msg_vec = self.log.get_local_messages_by_hash(&key_str);
        println!("###################Current Reply Messages#################");
        println!("{:?}", &msg_vec);
        println!("###############################################################");

        // check f+1
        let current_count = self.log.get_local_reply_messages_count_by_hash(&key_str);
        let reply_threshold = (self.state.node_count as u64 - self.state.fault_tolerance_count - 1) as u64;
        println!("cerren key is : {:?}",key_str.clone());
        println!("current_cous is : {:?},threshold is : {:?}",current_count,reply_threshold);
        if current_count as u64 == reply_threshold {
            println!("满足条件，发送切换requets‘");
            // self.state.client_state = ClientState::Replied;

            let request = Request {
                cmd: msg.clone().cmd,
                // timestamp: msg.clone().timestamp,
            };

            let data = ConsensusMessage{
                msg_type: MessageType::Request(request.clone()),
            };
            

        
            let result_str = std::str::from_utf8(&msg.result[..]).unwrap();
            println!("###############################################################");
            println!("Request consesnus successful, result is {}", result_str);

            self.log.reset();
            self.log.reply_messages.clear();
            
            return PhaseState::Over(None);
        }
        return PhaseState::ContinueExecute(send_query)
    }

    fn handle_checkpoint(&mut self, _current_peer_id: &[u8], msg: &CheckPoint) -> PhaseState {
        let mut send_query = VecDeque::new();
        // verify signature

        self.log
            .record_message_handler(MessageType::CheckPoint(msg.clone()));

        // check 2f+1 the same checkpoint message
        let current_checkpoint_count = self
            .log
            .get_checkpoint_count(msg.current_max_number, &msg.checkpoint_state_digest);
        let threshold = 2 * self.state.fault_tolerance_count;

        if current_checkpoint_count as u64 == threshold {
            self.state.stable_checkpoint.0 = msg.current_max_number;
            self.state.stable_checkpoint.1 = msg.checkpoint_state_digest.clone();
            // discard all pre-prepare, prepare, commit, checkpoint messages
            // with sequence number less than or equal to stable_checkpoint_number
            self.log
                .discard_messages_before_stable_checkpoint(self.state.stable_checkpoint.0);

            println!(
                "【Current Stable Checkpoint】 is {}",
                self.state.stable_checkpoint.0
            );
        }

        PhaseState::ContinueExecute(send_query)
    }

    // Consensus node handles the received VIEWCHANGE message
    pub fn handle_viewchange(&mut self, current_peer_id: &[u8], msg: &ViewChange) -> PhaseState {
        // verify signature
        let mut send_type = VecDeque::new();
        let new_view = msg.new_view;
        if new_view <= self.state.view {
            return PhaseState::ContinueExecute(send_type);
        }

        // record viewchange message
        self.log
            .record_message_handler(MessageType::ViewChange(msg.clone()));
        let msg_count = self
            .log
            .get_viewchange_messages_count_by_view(new_view);
        // let mode = *self.state.mode.lock().await;

        // The current node has not detected the exception, but has received F +1 viewchange message.
        // That is, the node also starts the viewchange, preventing the view switchover process from starting too late.
        if msg_count as u64 > self.state.fault_tolerance_count
            // && match mode {
            //     Mode::Abnormal => false,
            //     Mode::Normal => true,
            // }
        {
            return self.broadcast_viewchange(current_peer_id)
            
        }

        // When recieved 2f+1 viewchange, start viewchange timeout

        // The current node is the primary node and has received 2f+1 viewchange messages
        if self.state.is_primary && msg_count as u64 > 2 * self.state.fault_tolerance_count {
            return self.broadcast_newview(new_view)
        }
        else {
            return PhaseState::ContinueExecute(send_type)
        }
    }

    // The primary node creates a newview message based on the local storage messages
    // And broadcast it to other nodes to enter the next view
    pub fn broadcast_newview(&self, new_view: u64) -> PhaseState {
        let mut send_query = VecDeque::new();
        let viewchanges = *self.log.get_viewchange_messages_by_view(new_view);
        // let min = viewchanges.first().unwrap().latest_stable_checkpoint;
        let max = self
            .log
            .get_max_sequence_number_in_viewchange_by_view(new_view);
        let preprepares = *self
            .log
            .create_newview_preprepare_messages(new_view, 0, max);

        let newview = NewView {
            view: new_view,
            viewchanges,
            preprepares,
            signature: String::from(""),
        };
        let newview_msg = ConsensusMessage {
            msg_type: MessageType::NewView(newview),
        };
        let serialized_newview_msg = coder::serialize_into_bytes(&newview_msg);
        send_query.push_back(SendType::Broadcast(serialized_newview_msg));
        PhaseState::ContinueExecute(send_query)        
    }


    fn handle_newview(&self) -> PhaseState {
        let mut send_query = VecDeque::new();
        PhaseState::ContinueExecute(send_query)
    }

    pub fn distribute_public_key(&self, current_peer_id: &[u8]) -> PhaseState {
        println!("*********************** Distribute Public Key ****************************");
        let mut send_query = VecDeque::new();
        let eddsa_pk = self.keypair.get_public_key();
        let public_key = PublicKey {
            pk: eddsa_pk,
            from_peer_id: current_peer_id.to_vec(),
        };
        let msg = ConsensusMessage {
            msg_type: MessageType::PublicKey(public_key),
        };
        let serialized_msg = coder::serialize_into_bytes(&msg);
        // self.msg_tx.send(serialized_msg).await.unwrap();]
        
        send_query.push_back(SendType::Broadcast(serialized_msg));
        println!("*********************** Distribute Public Key Success!!! ****************************");
        PhaseState::ContinueExecute(send_query)
    }

    pub fn storage_public_key_by_peer_id(
        &mut self,
        public_key: &PublicKey,
    ) -> PhaseState {
        let mut send_query = VecDeque::new();
        
        println!("*********************** storage_public_key_by_peer_id  ****************************");
        let value = public_key.clone().pk.clone().0;
        self.consensus_node_pk
            .entry(public_key.clone().from_peer_id)
            .or_insert(value.clone());

        println!("{:?}",&public_key.from_peer_id);
        let peer = PeerId::from_bytes(&public_key.from_peer_id).expect("peer id bytes error.");
        println!("{}'s public key is {:?}", peer, value.clone());
        PhaseState::ContinueExecute(send_query)
    }
}

impl ProtocolBehaviour for PBFTProtocol {
    fn extra_initial_start(
        &mut self,
        consensus_nodes: HashSet<PeerId>,
        current_peer_id: Vec<u8>,
    ) -> PhaseState {
        let mut send_query = VecDeque::new();
        self.state.node_count = consensus_nodes.len() as u64;
        let msg = ConsensusMessage {
            msg_type: MessageType::DistributePK,
        };
        let serialized_msg = coder::serialize_into_bytes(&msg);
        
        send_query.push_back(SendType::Broadcast(serialized_msg));

        let data = self.distribute_public_key(&current_peer_id);
        if let PhaseState::ContinueExecute(msgs) = data {
            for msg in msgs {
                send_query.push_back(msg);
            }
        }
        PhaseState::ContinueExecute(send_query)
    }

    // fn receive_consensus_requests(&mut self, requests: Vec<Request>) {}

    fn consensus_protocol_message_handler(
        &mut self,
        _msg: &[u8],
        current_peer_id: Vec<u8>,
        peer_id: Option<PeerId>,
    ) -> PhaseState {
        let message: ConsensusMessage = coder::deserialize_for_bytes(_msg);
        match message.msg_type {
            MessageType::Request(msg) => {
                return self.handle_request(&current_peer_id, &msg)
            }
            MessageType::PrePrepare(msg) => {
                return self.handle_preprepare(&current_peer_id, &msg)
            }
            MessageType::Prepare(msg) => {
                return self.handle_prepare(&current_peer_id, &msg);
            }
            MessageType::Commit(msg) => {
                return self.handle_commit(&current_peer_id, &msg)
            }
            MessageType::Reply(msg) => {
                return self.handle_reply(&current_peer_id, &msg);
            }
            MessageType::CheckPoint(msg) => {
                return self.handle_checkpoint(&current_peer_id, &msg);
            }
            MessageType::ViewChange(msg) => {
                return self.handle_viewchange(&current_peer_id, &msg);
            }
            MessageType::NewView(msg) => {
                return self.handle_newview();
            }
            MessageType::PublicKey(msg) => {
                return self.storage_public_key_by_peer_id(&msg);
            }
            MessageType::DistributePK => {
                return self.distribute_public_key(&current_peer_id);
            }
        }

        // ConsensusEnd::No
        let mut queue = VecDeque::new();
        queue.push_back(SendType::Broadcast(vec![]));
        PhaseState::ContinueExecute(queue)
    }

    
    fn check_taken_request(&self,request:Vec<u8>) -> bool {
        if self.taken_requests.contains(&request) {
            true
        }
        else {
            false
        }
    }

    fn receive_consensus_requests(
        &mut self,
        requests: Vec<Request>,
    ) {
        todo!()
    }

    // fn view_timeout_handler(&mut self) {}

    fn init_timeout_notify(&mut self, tiemout_notify: Arc<Notify>) {
    }

    fn generate_serialized_request_message(&self, request: &Request) -> Vec<u8> {
        let msg = ConsensusMessage {
            msg_type: MessageType::Request(request.to_owned()),
        };
        let data = coder::serialize_into_bytes(&msg);
        data
    }

    
    fn current_request(&self) -> Request {
        todo!()
    }

    fn is_leader(&self, peer_id: Vec<u8>) -> bool {
        match self.state.primary.cmp(&peer_id) {
            Ordering::Equal => {
                println!("是leader");
                true
            }
            _ => false,
        }
    }

    fn view_timeout_handler(&mut self, current_peer_id: PeerId) -> PhaseState {
        println!("OK?");
        let current_view_timeout = self.current_view_timeout;
        self.current_view_timeout = current_view_timeout * 2;
        // send newview
        let msg = self.broadcast_viewchange(&current_peer_id.to_bytes());
        return msg;
    }

    fn get_current_phase(&mut self, _msg: &[u8]) -> u8 {
        1
    }

    fn protocol_reset(&mut self) {

    }

    fn protocol_phases(&mut self) -> HashMap<u8, Vec<u8>> {
        println!("No security test interface is implemented！");
        HashMap::new()
    }

    fn phase_map(&self) -> HashMap<u8,String> {
        HashMap::new()
    }

    fn set_leader(&mut self, is_leader: bool) {
        
    }

    

    fn set_current_request(&mut self, request: &Request) {}
}
