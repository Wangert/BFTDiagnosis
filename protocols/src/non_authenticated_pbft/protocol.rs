use crate::{non_authenticated_pbft::common::{get_message_key, STABLE_CHECKPOINT_DELTA}};
use libp2p::PeerId;
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    time::Duration, str::FromStr,
};
use super::{log::{ConsensusLog,LogPhaseState}, state::State};
use tokio::sync::{
    Notify,
};
use utils::{
    coder::{self, get_hash_str},
    crypto::eddsa::{EdDSAKeyPair},
};

use components::behaviour::{PhaseState, ProtocolBehaviour, SendType};
use crate::non_authenticated_pbft::message::ConsensusMessage;

use crate::non_authenticated_pbft::message::{
Commit, PrePrepare, Prepare, ProofMessages, Reply,
};
use components::message::Request;
use crate::non_authenticated_pbft::message::MessageType;

pub struct NonAuthPBFTProtocol {
    pub state: State,
    pub log: Box<ConsensusLog>,
    pub phase_map: HashMap<u8,String>,
    pub taken_requests: HashSet<Vec<u8>>,
    pub keypair: Box<EdDSAKeyPair>,
    pub viewchange_notify: Arc<Notify>,
    pub timeout_notify: Arc<Notify>,

    pub consensus_node_pk: HashMap<Vec<u8>, Vec<u8>>,
}

impl Default for NonAuthPBFTProtocol {
    fn default() -> Self {
        Self {
            state: State::new(Duration::from_secs(5)),
            log: Box::new(ConsensusLog::new()),
            phase_map: HashMap::new(),
            keypair: Box::new(EdDSAKeyPair::new()),
            viewchange_notify: Arc::new(Notify::new()),
            timeout_notify: Arc::new(Notify::new()),
            consensus_node_pk: HashMap::new(),
            taken_requests: HashSet::new(),
        }
    }
}

impl NonAuthPBFTProtocol {
    fn handle_request(&mut self, current_peer_id: &[u8], request: &Request) -> PhaseState {
        self.taken_requests.insert(coder::serialize_into_json_bytes(request));
        let mut send_query = VecDeque::new();
        self.state.primary = current_peer_id.to_vec();
        println!("******************* Handle request *******************");
        // create PrePrepare message
        let view = self.state.view;
        let seq_number = self.state.current_sequence_number + 1;

        let serialized_request = coder::serialize_into_json_bytes(&request);
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
            from_peer_id: current_peer_id.to_vec(),
        };

        

        let broadcast_msg = ConsensusMessage {
            msg_type: MessageType::PrePrepare(preprepare),
        };

        let serialized_msg = coder::serialize_into_json_bytes(&broadcast_msg);
        send_query.push_back(SendType::Broadcast(serialized_msg));
        PhaseState::ContinueExecute(send_query)
    }
    fn handle_preprepare(
        &mut self,
        current_peer_id: &[u8],
        msg: &PrePrepare,
    ) -> PhaseState {
        let mut send_query = VecDeque::new();
        let request:Request = coder::deserialize_for_json_bytes(&msg.m);
        self.taken_requests.insert(coder::serialize_into_json_bytes(&request));
        self.state.primary = msg.clone().from_peer_id;

        println!("*******************Handle Preprepare*******************");


        let key_str = get_message_key(&MessageType::PrePrepare(msg.clone()));
        //let source_pk = self.consensus_node_pk.get(&msg.from_peer_id);
        let peer = PeerId::from_bytes(&msg.from_peer_id).expect("peer bytes error.");
        

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
        let m: Request = coder::deserialize_for_json_bytes(&serialized_request[..]);
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
        };

        

        let broadcast_msg = ConsensusMessage {
            msg_type: MessageType::Prepare(prepare),
        };

        let serialized_msg = coder::serialize_into_json_bytes(&broadcast_msg);
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
        let peer = PeerId::from_bytes(&msg.from_peer_id).expect("peer bytes error.");
        


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
            };

            

            let broadcast_msg = ConsensusMessage {
                msg_type: MessageType::Commit(commit),
            };

            let serialized_msg = coder::serialize_into_json_bytes(&broadcast_msg);
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
        
        let peer = PeerId::from_bytes(&msg.from_peer_id).expect("peer bytes error.");
        

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
            println!("【Commit message to 2f+1, send reply message】");
            // create reply message
            let cmd = m.cmd;
            let view = self.state.view;
            let seq_number = msg.number;
            let mut reply = Reply {
                number: seq_number,
                from_peer_id: current_peer_id.to_vec(),
                result: "ok!".as_bytes().to_vec(),
                view,
                cmd:cmd.clone(),
            };

            let broadcast_msg = ConsensusMessage {
                msg_type: MessageType::Reply(reply),
            };
            let serialized_msg = coder::serialize_into_json_bytes(&broadcast_msg);
            
            send_query.push_back(SendType::Unicast(PeerId::from_bytes(&self.state.primary).expect("msg"), serialized_msg));

            let request = Request {
                cmd:cmd.clone(),
            };
            let msg = ConsensusMessage {
                msg_type: MessageType::Request(request.clone()),
            };
            
            self.log.reset();
            return PhaseState::Complete(request, send_query);
            
        }
        PhaseState::ContinueExecute(send_query)
    }

    fn handle_reply(&mut self, _current_peer_id: &[u8], msg: &Reply) -> PhaseState {
        let mut send_query = VecDeque::new();
        let key_str = get_message_key(&&MessageType::Reply(msg.clone()));
        let threshold = (self.state.node_count as u64 - self.state.fault_tolerance_count) as u64;


        println!("******************* Handle Reply *******************");
        // verify signature
        let key_str = get_message_key(&MessageType::Reply(msg.clone()));
        let peer = PeerId::from_bytes(&msg.from_peer_id).expect("peer bytes error.");
        

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
}



    
impl ProtocolBehaviour for NonAuthPBFTProtocol {
    fn extra_initial_start(
        &mut self,
        consensus_nodes: HashSet<PeerId>,
        current_peer_id: Vec<u8>,
        analyzer_id: String,
    ) -> PhaseState {
        self.state.primary = current_peer_id.clone();
        let mut send_query = VecDeque::new();
        self.state.node_count = consensus_nodes.len() as u64;
        PhaseState::OverMessage(None,send_query)
    }

    // fn receive_consensus_requests(&mut self, requests: Vec<Request>) {}

    fn consensus_protocol_message_handler(
        &mut self,
        _msg: &[u8],
        current_peer_id: Vec<u8>,
        peer_id: Option<PeerId>,
    ) -> PhaseState {
        let message: ConsensusMessage = coder::deserialize_for_json_bytes(_msg);
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
            
        }

        // ConsensusEnd::No
        let mut queue = VecDeque::new();
        queue.push_back(SendType::Broadcast(vec![]));
        PhaseState::ContinueExecute(queue)
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
        let data = coder::serialize_into_json_bytes(&msg);
        data
    }

    fn phase_map(&self) -> HashMap<u8,String> {
        self.phase_map.clone()
    }

    fn protocol_phases(&mut self) -> HashMap<u8, Vec<u8>> {
        let mut hash_map = HashMap::new();
        let prepare = Prepare {
            view: 1,
            number: 1,
            m_hash: String::from(""),
            from_peer_id: vec![],
        };
        let commit = Commit {
            view: 1,
            number: 2,
            m_hash: String::from(""),
            from_peer_id: vec![],
        };
        let prepare_json = coder::serialize_into_json_str(&prepare).as_bytes().to_vec();
        let commit_json = coder::serialize_into_json_str(&commit).as_bytes().to_vec();
        hash_map.insert(1, prepare_json);
        hash_map.insert(2, commit_json);
        
        self.phase_map.insert(1, String::from("Prepare"));
        self.phase_map.insert(2, String::from("Commit"));

        hash_map
    }

    fn get_current_phase(&mut self, _msg: &[u8]) -> u8 {
        if _msg.len() == 0 {
            return 0
        }
        else {
            let data: ConsensusMessage = coder::deserialize_for_json_bytes(_msg);
        let i = match data.msg_type {
            MessageType::Request(_) => 0,
            MessageType::PrePrepare(_) => 0,
            MessageType::Prepare(_) => 1,
            MessageType::Commit(_) => 2,
            MessageType::Reply(_) => 0,
            _ => 0
        };
        return i;
        }
    }



    fn current_request(&self) -> Request {
        todo!()
    }

    fn check_taken_request(&self,request:Vec<u8>) -> bool {
        if self.taken_requests.contains(&request) {
            true
        }
        else {
            false
        }
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
}
    

    

    


