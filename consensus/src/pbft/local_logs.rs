use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use libp2p::futures::future::Map;
use tokio::sync::Mutex;

use super::{
    common::{
        self, get_commit_key_by_request_hash, get_prepare_key_by_request_hash,
        get_preprepare_key_by_request_hash,
    },
    message::{CheckPoint, Commit, MessageType, PrePrepare, Prepare, Reply, Request, ViewChange},
    state::PhaseState,
};

// node's local logs in consensus process
pub struct LocalLogs {
    pub requests: HashMap<u64, Request>,
    pub messages: HashMap<String, Vec<MessageType>>,
    // (sequence_number, checkpoint_state_digest) -> checkpoint messages
    pub checkpoints: HashMap<(u64, String), Vec<CheckPoint>>,
    pub viewchanges: HashMap<u64, Vec<ViewChange>>,
    // request_key -> (view, sequence_number)
    pub request_map: HashMap<String, (u64, u64)>,
    pub request_phase_state: HashMap<String, PhaseState>,
    // record request start timestamp
    pub current_requests: Arc<Mutex<VecDeque<(String, Instant)>>>,
}

impl LocalLogs {
    pub fn new() -> LocalLogs {
        let requests: HashMap<u64, Request> = HashMap::new();
        let messages: HashMap<String, Vec<MessageType>> = HashMap::new();
        let checkpoints: HashMap<(u64, String), Vec<CheckPoint>> = HashMap::new();
        let viewchanges: HashMap<u64, Vec<ViewChange>> = HashMap::new();
        let request_map: HashMap<String, (u64, u64)> = HashMap::new();
        let request_phase_state: HashMap<String, PhaseState> = HashMap::new();
        let current_requests: Arc<Mutex<VecDeque<(String, Instant)>>> =
            Arc::new(Mutex::new(VecDeque::new()));

        LocalLogs {
            requests,
            messages,
            checkpoints,
            viewchanges,
            request_map,
            request_phase_state,
            current_requests,
        }
    }

    pub fn record_message_handler(&mut self, msg: MessageType) {
        match msg {
            MessageType::PrePrepare(preprepare) => {
                self.record_preprepare(&preprepare);
            }
            MessageType::Prepare(prepare) => {
                self.record_prepare(&prepare);
            }
            MessageType::Commit(commit) => {
                self.record_commit(&commit);
            }
            MessageType::Reply(reply) => {
                self.record_reply(&reply);
            }
            MessageType::CheckPoint(checkpoint) => {
                self.record_checkpoint(&checkpoint);
            }
            MessageType::ViewChange(viewchange) => {
                self.record_viewchange(&viewchange);
            }
            _ => {}
        }
    }

    // record viewchange messages
    pub fn record_viewchange(&mut self, viewchange: &ViewChange) {
        let new_view = viewchange.new_view;
        if let Some(viewchange_vec) = self.viewchanges.get_mut(&new_view) {
            viewchange_vec.push(viewchange.clone());
        } else {
            self.viewchanges.insert(new_view, vec![viewchange.clone()]);
        }
    }

    // get viewchange messages by view number
    pub fn get_viewchange_messages_by_view(&self, view: u64) -> Box<Vec<ViewChange>> {
        if let Some(viewchange_vec) = self.viewchanges.get(&view) {
            Box::new(viewchange_vec.clone())
        } else {
            Box::new(vec![])
        }
    }

    // discard all pre-prepare, prepare, commit, checkpoint messages
    // with sequence number less than or equal to stable_checkpoint_number
    pub fn discard_messages_before_stable_checkpoint(&mut self, stable_checkpoint_number: u64) {
        // get pre-prepare, prepare and commit message key
        let message_keys: Vec<(String, String, String)> = self
            .request_map
            .iter()
            .filter(|(_, (_, sequence_number))| *sequence_number <= stable_checkpoint_number)
            .map(|(request_hash, (view, sequence_number))| {
                let preprepare_key = get_preprepare_key_by_request_hash(
                    request_hash.as_bytes(),
                    *view,
                    *sequence_number,
                );
                let prepare_key = get_prepare_key_by_request_hash(
                    request_hash.as_bytes(),
                    *view,
                    *sequence_number,
                );
                let commit_key = get_commit_key_by_request_hash(
                    request_hash.as_bytes(),
                    *view,
                    *sequence_number,
                );

                (preprepare_key, prepare_key, commit_key)
            })
            .collect();

        let checkpoint_keys: Vec<(u64, String)> = self
            .checkpoints
            .iter()
            .filter(|((sequence_number, _), _)| *sequence_number < stable_checkpoint_number)
            .map(|((sequence_number, digest), _)| (*sequence_number, digest.to_string()))
            .collect();

        for (key1, key2, key3) in message_keys {
            self.messages.remove(&key1);
            self.messages.remove(&key2);
            self.messages.remove(&key3);
        }
        for key in checkpoint_keys {
            self.checkpoints.remove(&key);
        }
    }

    pub fn get_checkpoint_messages_by_sequence_number(
        &self,
        sequence_number: u64,
        threshold: u64,
    ) -> Box<Vec<CheckPoint>> {
        let mut checkpoint_vec_iter = self
            .checkpoints
            .iter()
            .filter(|((seq_number, _), checkpoint_vec)| {
                *seq_number == sequence_number && checkpoint_vec.len() as u64 == threshold
            });
        let checkpoint_vec = checkpoint_vec_iter.next();
        if let Some(checkpoint) = checkpoint_vec {
            Box::new(checkpoint.1.clone())
        } else {
            Box::new(vec![])
        }
    }

    // When a view change occurs
    // Need to send preprepare and prepare messages for pending requests
    pub fn get_preprepare_and_prepare_by_request_hash(
        &self,
        request_hash: &str,
    ) -> Box<Vec<MessageType>> {
        let (view, sequence_number) = self.request_map.get(request_hash).expect("No request!");
        let preprepare_key =
            get_preprepare_key_by_request_hash(request_hash.as_bytes(), *view, *sequence_number);
        let prepare_key =
            get_prepare_key_by_request_hash(request_hash.as_bytes(), *view, *sequence_number);

        let preprepare_vec = if let Some(preprepare_vec) = self.messages.get(&preprepare_key) {
            preprepare_vec.clone()
        } else {
            vec![]
        };
        let prepare_vec = if let Some(prepare_vec) = self.messages.get(&prepare_key) {
            prepare_vec.clone()
        } else {
            vec![]
        };

        Box::new([preprepare_vec, prepare_vec].concat())
    }

    pub async fn get_proof_messages(&self) -> Box<Vec<MessageType>> {
        let mut current_requests = self.current_requests.lock().await.clone();
        let mut proof_messages: Vec<MessageType> = vec![];
        loop {
            if let Some(v) = current_requests.pop_front() {
                let mut p = *self.get_preprepare_and_prepare_by_request_hash(&v.0).clone();
                proof_messages.append(&mut p);
            } else {
                break;
            };
        }
        Box::new(proof_messages)
    }

    // Record a request that do not reach a consensus
    pub async fn record_current_request(&mut self, request_key: &str) {
        self.current_requests
            .lock()
            .await
            .push_back((request_key.to_string(), Instant::now()));
    }

    // Remove a consensus request
    pub async fn remove_commited_request(&mut self) {
        self.current_requests.lock().await.pop_front();
    }

    pub fn update_request_phase_state(&mut self, request_key: &str, state: PhaseState) {
        self.request_phase_state
            .insert(request_key.to_string(), state);
    }

    // get request current phase state
    pub fn get_request_phase_state(&mut self, request_key: &str) -> Option<&PhaseState> {
        self.request_phase_state.get(request_key)
    }

    pub fn record_request(&mut self, sequence_number: u64, request: &Request) {
        if !self.requests.contains_key(&sequence_number) {
            self.requests.insert(sequence_number, request.clone());
        }
    }

    pub fn record_checkpoint(&mut self, msg: &CheckPoint) {
        let key = (msg.current_max_number, msg.checkpoint_state_digest.clone());
        if let Some(msg_vec) = self.checkpoints.get_mut(&key) {
            msg_vec.push(msg.clone());
        } else {
            let msg_vec = vec![msg.clone()];
            self.checkpoints.insert(key, msg_vec);
        }
    }

    // get checkpoint message count by sequence_number and checkpoint state digest
    pub fn get_checkpoint_count(
        &self,
        sequence_number: u64,
        checkpoint_state_digest: &str,
    ) -> usize {
        let key = (sequence_number, checkpoint_state_digest.to_string());
        if let Some(msg_vec) = self.checkpoints.get(&key) {
            msg_vec.len()
        } else {
            0
        }
    }

    pub fn record_preprepare(&mut self, msg: &PrePrepare) {
        let key_hash = common::get_message_key(MessageType::PrePrepare(msg.clone()));

        //println!("[Preprepare hash key]：{:?}", &key_hash);

        if let Some(msg_vec) = self.messages.get_mut(&key_hash) {
            msg_vec.push(MessageType::PrePrepare(msg.clone()));
        } else {
            let msg_vec = vec![MessageType::PrePrepare(msg.clone())];
            self.messages.insert(key_hash, msg_vec);
        }
    }

    pub fn record_prepare(&mut self, msg: &Prepare) {
        let key_hash = common::get_message_key(MessageType::Prepare(msg.clone()));

        //println!("[Prepare hash key]：{:?}", &key_hash);

        if let Some(msg_vec) = self.messages.get_mut(&key_hash) {
            msg_vec.push(MessageType::Prepare(msg.clone()));
        } else {
            let msg_vec = vec![MessageType::Prepare(msg.clone())];
            self.messages.insert(key_hash, msg_vec);
        }
    }

    pub fn record_commit(&mut self, msg: &Commit) {
        let key_hash = common::get_message_key(MessageType::Commit(msg.clone()));

        //println!("[Preprepare hash key]：{:?}", &key_hash);

        if let Some(msg_vec) = self.messages.get_mut(&key_hash) {
            msg_vec.push(MessageType::Commit(msg.clone()));
        } else {
            let msg_vec = vec![MessageType::Commit(msg.clone())];
            self.messages.insert(key_hash, msg_vec);
        }
    }

    pub fn record_reply(&mut self, msg: &Reply) {
        let key_hash = common::get_message_key(MessageType::Reply(msg.clone()));

        if let Some(msg_vec) = self.messages.get_mut(&key_hash) {
            msg_vec.push(MessageType::Reply(msg.clone()))
        } else {
            let msg_vec = vec![MessageType::Reply(msg.clone())];
            self.messages.insert(key_hash, msg_vec);
        }
    }

    pub fn get_local_request_by_sequence_number(&self, sequence_number: u64) -> Option<&Request> {
        self.requests.get(&sequence_number)
    }

    pub fn get_local_messages_by_hash(&self, hash: &str) -> Box<Vec<MessageType>> {
        if let Some(msg_vec) = self.messages.get(&hash.to_string()) {
            Box::new(msg_vec.clone())
        } else {
            Box::new(vec![])
        }
    }

    pub fn get_local_messages_count_by_hash(&self, hash: &str) -> usize {
        if let Some(msg_vec) = self.messages.get(&hash.to_string()) {
            msg_vec.len()
        } else {
            0
        }
    }
}
