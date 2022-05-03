use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Notify,
};
use utils::coder::{self, get_hash_str};

use crate::pbft::{
    common::{get_message_key, STABLE_CHECKPOINT_DELTA},
    state::{ClientState, PhaseState},
};

use super::{
    local_logs::LocalLogs,
    message::{
        CheckPoint, Commit, Message, MessageType, NewView, PrePrepare, Prepare, Reply, Request,
        ViewChange,
    },
    state::{Mode, State},
};

// Node consensus executor
pub struct Executor {
    pub state: State,
    pub local_logs: Box<LocalLogs>,
    pub msg_tx: Sender<Vec<u8>>,
    pub msg_rx: Receiver<Vec<u8>>,
    pub viewchange_notify: Arc<Notify>,
    pub timeout_notify: Arc<Notify>,
}

impl Executor {
    pub fn new() -> Executor {
        let (msg_tx, msg_rx) = mpsc::channel::<Vec<u8>>(10);

        Executor {
            state: State::new(Duration::from_secs(5)),
            local_logs: Box::new(LocalLogs::new()),
            msg_tx,
            msg_rx,
            viewchange_notify: Arc::new(Notify::new()),
            timeout_notify: Arc::new(Notify::new()),
        }
    }

    pub async fn pbft_message_handler(&mut self, source: &str, msg: &Vec<u8>) {
        //if let Some(msg) = self.message_rx.recv().await {
        //let msg = msg.as_bytes();
        let message: Message = coder::deserialize_for_bytes(msg);
        let mode_clone = self.state.mode.clone();
        let current_mode = *mode_clone.lock().await;
        match current_mode {
            Mode::Normal => match message.msg_type {
                MessageType::Request(m) => {
                    self.handle_request(&m).await;
                }
                MessageType::PrePrepare(m) => {
                    self.handle_preprepare(source, &m).await;
                }
                MessageType::Prepare(m) => {
                    self.handle_prepare(source, &m).await;
                }
                MessageType::Commit(m) => {
                    self.handle_commit(source, &m).await;
                }
                MessageType::Reply(m) => {
                    self.handle_reply(source, &m);
                }
                MessageType::CheckPoint(m) => {
                    self.handle_checkpoint(source, &m);
                }
                MessageType::ViewChange(m) => {
                    self.handle_viewchange(source, &m).await;
                }
                MessageType::NewView(m) => {
                    self.handle_newview(source, &m).await;
                }
            },
            Mode::Abnormal => match message.msg_type {
                MessageType::CheckPoint(m) => {
                    self.handle_checkpoint(source, &m);
                }
                MessageType::ViewChange(m) => {
                    self.handle_viewchange(source, &m).await;
                }
                MessageType::NewView(m) => {
                    self.handle_newview(source, &m).await;
                }
                _ => {}
            },
        }
    }

    pub fn timeout_check_start(&self) {
        println!("==============【timeout check start】==============");
        let requests_start = self.local_logs.current_requests.clone();
        let timeout_duration = self.state.commited_timeout.duration;
        let mode = self.state.mode.clone();
        let notify = self.viewchange_notify.clone();
        tokio::spawn(async move {
            loop {
                // Avoid broadcast viewchange messages again
                if let Mode::Abnormal = *mode.lock().await {
                    continue;
                }

                let current_requests_queue = requests_start.lock().await.clone();
                let first_request = current_requests_queue.front();
                if let Some(request) = first_request {
                    if request.1.elapsed() > timeout_duration {
                        println!("{} is timeout!", request.0);
                        notify.notify_one();
                    }
                }
            }
        });
    }

    // pub fn stable_checkpoint_update_start(&self) {
    //     println!("==============【stable checkpoint update start】==============");
    //     let count = self.state.current_commited_request_count.clone();
    //     tokio::spawn(async move {
    //         loop {
    //             if *count.lock().await >= STABLE_CHECKPOINT_DELTA {
    //                 // broadcast checkpoint message
    //                 todo!();
    //             }
    //         }
    //     });
    // }

    pub async fn broadcast_preprepare(&self, msg: &[u8]) {
        self.msg_tx.send(msg.to_vec()).await.unwrap();
    }

    pub async fn broadcast_prepare(&self, msg: &[u8]) {
        self.msg_tx.send(msg.to_vec()).await.unwrap();
    }

    pub async fn broadcast_commit(&self, msg: &[u8]) {
        self.msg_tx.send(msg.to_vec()).await.unwrap();
    }

    pub async fn reply(&self, msg: &[u8]) {
        self.msg_tx.send(msg.to_vec()).await.unwrap();
    }

    pub async fn broadcast_checkpoint(&self, msg: &[u8]) {
        self.msg_tx.send(msg.to_vec()).await.unwrap();
    }

    // Create a viewchange message based on the local storage messages
    // And broadcast it to other nodes to enter the viewchange mode
    pub async fn broadcast_viewchange(&self) {
        let mode = self.state.mode.clone();
        // Avoid broadcast viewchange messages again
        if let Mode::Abnormal = *mode.lock().await {
            return;
        }
        // Switch to viewchange mode
        *mode.lock().await = Mode::Abnormal;
        let m = mode.lock().await.clone();
        println!("&&&&&&&&&&&&&&{:?}&&&&&&&&&&&&&&&", m);

        let threshold = 2 * self.state.fault_tolerance_count;
        let sequence_number = self.state.stable_checkpoint.0;
        let latest_stable_checkpoint_messages = *self
            .local_logs
            .get_checkpoint_messages_by_sequence_number(sequence_number, threshold)
            .clone();
        let proof_messages = *self.local_logs.get_proof_messages().await.clone();
        let viewchange = ViewChange {
            new_view: self.state.view + 1,
            latest_stable_checkpoint: self.state.stable_checkpoint.0,
            latest_stable_checkpoint_messages,
            proof_messages,
            from_peer_id: String::from(""),
            signature: String::from(""),
        };
        let viewchange_msg = Message {
            msg_type: MessageType::ViewChange(viewchange),
        };
        let serialized_viewchange_msg = coder::serialize_into_bytes(&viewchange_msg);
        self.msg_tx.send(serialized_viewchange_msg).await.unwrap();
    }

    // The primary node creates a newview message based on the local storage messages
    // And broadcast it to other nodes to enter the next view
    pub async fn broadcast_newview(&self, new_view: u64) {
        let viewchanges = *self.local_logs.get_viewchange_messages_by_view(new_view);
        let min = viewchanges.first().unwrap().latest_stable_checkpoint;
        let max = self
            .local_logs
            .get_max_sequence_number_in_viewchange_by_view(new_view);
        let preprepares = *self
            .local_logs
            .create_newview_preprepare_messages(new_view, min, max);

        let newview = NewView {
            view: new_view,
            viewchanges,
            preprepares,
            signature: String::from(""),
        };
        let newview_msg = Message {
            msg_type: MessageType::NewView(newview),
        };
        let serialized_newview_msg = coder::serialize_into_bytes(&newview_msg);
        self.msg_tx.send(serialized_newview_msg).await.unwrap();
    }

    // Consensus node handles the received REQUEST message
    pub async fn handle_request(&mut self, r: &Request) {
        // verify request message(client signature)

        // generate number for request

        println!("handle_request ok!");

        // create PrePrepare message
        let view = self.state.view;
        let seq_number = self.state.current_sequence_number + 1;

        let serialized_request = coder::serialize_into_bytes(&r);
        self.local_logs.record_request(seq_number, &r.clone());
        let request = self
            .local_logs
            .get_local_request_by_sequence_number(seq_number);
        println!("###################Current Request Messages#################");
        println!("{:?}", &request);
        println!("###############################################################");

        let m_hash = get_hash_str(&serialized_request);
        // signature
        let signature = String::from("");

        //let id = self.peer_id.clone();
        let preprepare = PrePrepare {
            view,
            number: seq_number,
            m_hash,
            m: serialized_request,
            signature,
            from_peer_id: String::from(""),
        };

        let broadcast_msg = Message {
            msg_type: MessageType::PrePrepare(preprepare),
        };

        let serialized_msg = coder::serialize_into_bytes(&broadcast_msg);
        //let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

        // broadcast PrePrepare message
        self.broadcast_preprepare(&serialized_msg[..]).await;
    }

    // Consensus node handles the received PREPREPARE message
    pub async fn handle_preprepare(&mut self, source: &str, msg: &PrePrepare) {
        // verify preprepare message
        // check sequence number
        let low = self.state.stable_checkpoint.0;
        let high = self.state.stable_checkpoint.0 + 100;
        if msg.number <= low || msg.number > high {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return;
        }
        // check view
        if self.state.view != msg.view {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return;
        }

        // record request message
        let serialized_request = msg.m.clone();
        let m: Request = coder::deserialize_for_bytes(&serialized_request[..]);
        self.local_logs.record_request(msg.number, &m);
        let request = self
            .local_logs
            .get_local_request_by_sequence_number(msg.number);
        println!("###################Current Request Messages#################");
        println!("{:?}", &request);
        println!("###############################################################");

        // record preprepare message
        let mut record_msg = msg.clone();
        record_msg.from_peer_id = source.to_string();
        self.local_logs
            .record_message_handler(MessageType::PrePrepare(record_msg));
        let key_str = get_message_key(MessageType::PrePrepare(msg.clone()));

        let msg_vec = self.local_logs.get_local_messages_by_hash(&key_str);
        println!("###################Current PrePrepare Messages#################");
        println!("{:?}", &msg_vec);
        println!("###############################################################");

        // create prepare message
        let view = self.state.view;
        let seq_number = msg.number;
        let m_hash = msg.m_hash.clone();
        let signature = String::from("");
        let prepare = Prepare {
            view,
            number: seq_number,
            m_hash,
            from_peer_id: String::from(""),
            signature,
        };

        let broadcast_msg = Message {
            msg_type: MessageType::Prepare(prepare),
        };

        let serialized_msg = coder::serialize_into_bytes(&broadcast_msg);
        //let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

        // broadcast prepare message
        self.broadcast_prepare(&serialized_msg[..]).await;

        // record current request consensus start time
        self.local_logs.record_current_request(&msg.m_hash).await;
    }

    // Consensus node handles the received PREPARE message
    pub async fn handle_prepare(&mut self, source: &str, msg: &Prepare) {
        // check current request prepared state
        let phase_state = self.local_logs.get_request_phase_state(&msg.m_hash);
        if let Some(&PhaseState::Prepared) = phase_state {
            return;
        }

        // verify prepare message
        // check sequence number
        let low = self.state.stable_checkpoint.0;
        let high = self.state.stable_checkpoint.0 + 100;
        if msg.number <= low || msg.number > high {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return;
        }
        // check view
        if self.state.view != msg.view {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return;
        }

        // check m hash
        let m = self
            .local_logs
            .get_local_request_by_sequence_number(msg.number);
        if let Some(m) = m {
            let request_hash = get_message_key(MessageType::Request(m.clone()));
            if request_hash.ne(&msg.m_hash) {
                eprintln!("Request hash error!");
                return;
            }
        } else {
            eprintln!("Request is not exsit!");
            return;
        }

        // record message
        let mut record_msg = msg.clone();
        record_msg.from_peer_id = source.to_string();
        self.local_logs
            .record_message_handler(MessageType::Prepare(record_msg));

        let key_str = get_message_key(MessageType::Prepare(msg.clone()));
        let msg_vec = self.local_logs.get_local_messages_by_hash(&key_str);
        println!("###################Current Prepare Messages#################");
        println!("{:?}", &msg_vec);
        println!("###############################################################");

        // check 2f+1 prepare messages (include current node)
        let current_count = self.local_logs.get_local_messages_count_by_hash(&key_str);
        let threshold = 2 * self.state.fault_tolerance_count;
        if current_count as u64 == threshold {
            self.local_logs
                .update_request_phase_state(&msg.m_hash, PhaseState::Prepared);
            //self.state.phase_state = PhaseState::Prepared;
            println!("【Prepare message to 2f+1, send commit message】");
            // create commit message
            let view = self.state.view;
            let seq_number = msg.number;
            let m_hash = msg.m_hash.clone();
            let signature = String::from("");
            let commit = Commit {
                view,
                number: seq_number,
                m_hash,
                from_peer_id: String::from(""),
                signature,
            };

            let broadcast_msg = Message {
                msg_type: MessageType::Commit(commit),
            };

            let serialized_msg = coder::serialize_into_bytes(&broadcast_msg);
            //let str_msg = std::str::from_utf8(&serialized_msg).unwrap();

            // broadcast commit message
            self.broadcast_commit(&serialized_msg[..]).await;
        }
    }

    // Consensus node handles the received COMMIT message
    pub async fn handle_commit(&mut self, source: &str, msg: &Commit) {
        // check current request commited state
        let phase_state = self.local_logs.get_request_phase_state(&msg.m_hash);
        if let Some(&PhaseState::Commited) = phase_state {
            return;
        }

        // verify commit message
        // check sequence number
        let low = self.state.stable_checkpoint.0;
        let high = self.state.stable_checkpoint.0 + 100;
        if msg.number <= low || msg.number > high {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return;
        }
        // check view
        if self.state.view != msg.view {
            eprintln!("Invalid preprepare message: {:?}", &msg);
            return;
        }

        // check m hash
        let m = self
            .local_logs
            .get_local_request_by_sequence_number(msg.number);
        let m = if let Some(m) = m {
            m.clone()
        } else {
            eprintln!("Not have local request record!");
            return;
        };

        // check m hash
        let request_hash = get_message_key(MessageType::Request(m.clone()));
        if request_hash.ne(&msg.m_hash) {
            eprintln!("Request hash error!");
            return;
        }

        // record message
        let mut record_msg = msg.clone();
        record_msg.from_peer_id = source.to_string();
        self.local_logs
            .record_message_handler(MessageType::Commit(record_msg));

        let key_str = get_message_key(MessageType::Commit(msg.clone()));
        let msg_vec = self.local_logs.get_local_messages_by_hash(&key_str);
        println!("###################Current Commit Messages#################");
        println!("{:?}", &msg_vec);
        println!("###############################################################");

        // check 2f+1 commit
        let current_count = self.local_logs.get_local_messages_count_by_hash(&key_str);
        let threshold = 2 * self.state.fault_tolerance_count;

        if current_count as u64 == threshold {
            // update checkpoint relate information
            self.state.update_checkpoint_state(&msg.m_hash);
            self.local_logs.remove_commited_request().await;
            *self.state.current_commited_request_count.lock().await += 1;
            let current_commited_request_count =
                *self.state.current_commited_request_count.lock().await;
            if current_commited_request_count != 0
                && (current_commited_request_count % STABLE_CHECKPOINT_DELTA) == 0
            {
                // broadcast checkpoint message
                let checkpoint = CheckPoint {
                    current_max_number: self.state.stable_checkpoint.0 + STABLE_CHECKPOINT_DELTA,
                    checkpoint_state_digest: self.state.checkpoint_state.clone(),
                    from_peer_id: String::from(""),
                    signature: String::from(""),
                };
                let checkpoint_msg = Message {
                    msg_type: MessageType::CheckPoint(checkpoint),
                };
                let serialized_checkpoint_msg = coder::serialize_into_bytes(&checkpoint_msg);
                self.broadcast_checkpoint(&serialized_checkpoint_msg[..])
                    .await;
            }

            self.local_logs
                .update_request_phase_state(&msg.m_hash, PhaseState::Commited);
            //self.state.phase_state = PhaseState::Commited;
            println!("【Commit message to 2f+1, send reply message】");
            // create reply message
            let view = self.state.view;
            let seq_number = msg.number;
            let signature = String::from("");

            let client_id = m.client_id;
            let timestamp = m.timestamp;

            let reply = Reply {
                client_id,
                timestamp,
                number: seq_number,
                from_peer_id: String::from(""),
                signature,
                result: "ok!".as_bytes().to_vec(),
                view,
            };
            let broadcast_msg = Message {
                msg_type: MessageType::Reply(reply),
            };
            let serialized_msg = coder::serialize_into_bytes(&broadcast_msg);
            // Send Reply message to client
            self.reply(&serialized_msg[..]).await;
        }
    }

    // Client handles the recevied REPLY message
    pub fn handle_reply(&mut self, source: &str, msg: &Reply) {
        if let ClientState::Replied = self.state.client_state {
            return;
        }
        // verify signature

        // record reply
        let mut record_msg = msg.clone();
        record_msg.from_peer_id = source.to_string();

        self.local_logs
            .record_message_handler(MessageType::Reply(record_msg));
        let key_str = get_message_key(MessageType::Reply(msg.clone()));
        let msg_vec = self.local_logs.get_local_messages_by_hash(&key_str);
        println!("###################Current Reply Messages#################");
        println!("{:?}", &msg_vec);
        println!("###############################################################");

        // check f+1
        let current_count = self.local_logs.get_local_messages_count_by_hash(&key_str);
        let reply_threshold = self.state.fault_tolerance_count + 1;
        if current_count as u64 == reply_threshold {
            self.state.client_state = ClientState::Replied;

            let result_str = std::str::from_utf8(&msg.result[..]).unwrap();
            println!("###############################################################");
            println!("Request consesnus successful, result is {}", result_str);
        }
    }

    // Consensus node handles the received CHECKPOINT message
    pub fn handle_checkpoint(&mut self, source: &str, msg: &CheckPoint) {
        // verify signature

        let mut checkpoint_msg = msg.clone();
        checkpoint_msg.from_peer_id = source.to_string();
        self.local_logs
            .record_message_handler(MessageType::CheckPoint(checkpoint_msg));

        // check 2f+1 the same checkpoint message
        let current_checkpoint_count = self
            .local_logs
            .get_checkpoint_count(msg.current_max_number, &msg.checkpoint_state_digest);
        let threshold = 2 * self.state.fault_tolerance_count;

        if current_checkpoint_count as u64 == threshold {
            self.state.stable_checkpoint.0 = msg.current_max_number;
            self.state.stable_checkpoint.1 = msg.checkpoint_state_digest.clone();
            // discard all pre-prepare, prepare, commit, checkpoint messages
            // with sequence number less than or equal to stable_checkpoint_number
            self.local_logs
                .discard_messages_before_stable_checkpoint(self.state.stable_checkpoint.0);

            println!(
                "【Current Stable Checkpoint】 is {}",
                self.state.stable_checkpoint.0
            );
        }
    }

    // Consensus node handles the received VIEWCHANGE message
    pub async fn handle_viewchange(&mut self, source: &str, msg: &ViewChange) {
        // verify signature

        let new_view = msg.new_view;
        if new_view <= self.state.view {
            return;
        }

        let mut viewchange = msg.clone();
        viewchange.from_peer_id = source.to_string();
        // record viewchange message
        self.local_logs
            .record_message_handler(MessageType::ViewChange(viewchange));
        let msg_count = self
            .local_logs
            .get_viewchange_messages_count_by_view(new_view);
        let mode = *self.state.mode.lock().await;

        // The current node has not detected the exception, but has received F +1 viewchange message.
        // That is, the node also starts the viewchange, preventing the view switchover process from starting too late.
        if msg_count as u64 > self.state.fault_tolerance_count
            && match mode {
                Mode::Abnormal => false,
                Mode::Normal => true,
            }
        {
            self.broadcast_viewchange().await;
            return;
        }

        // When recieved 2f+1 viewchange, start viewchange timeout

        // The current node is the primary node and has received 2f+1 viewchange messages
        if self.state.is_primary && msg_count as u64 > 2 * self.state.fault_tolerance_count {
            self.broadcast_newview(new_view).await;
        }
    }

    // The replic handles a newview message from the primary
    pub async fn handle_newview(&mut self, _source: &str, msg: &NewView) {
        // verify newview message
        if !self.verify_newview(msg) {
            eprintln!("NewView messages is invalid!");
            return;
        }

        // update view
        self.state.view = msg.view;
        // stop viewchange timeout

        // clear the cache of unexecuted requests
        self.local_logs.clear_current_request().await;
        *self.state.mode.lock().await = Mode::Normal;

        for preprepare in &msg.preprepares {
            let request_key = preprepare.m_hash.clone();
            // record preprepare message
            let record_msg = preprepare.clone();
            self.local_logs
                .record_message_handler(MessageType::PrePrepare(record_msg));
            let key_str = get_message_key(MessageType::PrePrepare(preprepare.clone()));

            let msg_vec = self.local_logs.get_local_messages_by_hash(&key_str);
            println!("###################Current PrePrepare Messages#################");
            println!("{:?}", &msg_vec);
            println!("###############################################################");

            // create prepare message
            let view = msg.view;
            let seq_number = preprepare.number;
            let m_hash = preprepare.m_hash.clone();
            let signature = String::from("");
            let prepare = Prepare {
                view,
                number: seq_number,
                m_hash,
                from_peer_id: String::from(""),
                signature,
            };

            let prepare_msg = MessageType::Prepare(prepare);
            let serialized_prepare_msg = coder::serialize_into_bytes(&prepare_msg);
            self.broadcast_prepare(&serialized_prepare_msg).await;
            self.local_logs.record_current_request(&request_key).await;
        }
    }

    pub fn verify_preprepare(&self, _preprepare: &PrePrepare) -> bool {
        // verify preprepare signature
        todo!();
    }

    pub fn verify_newview(&self, newview: &NewView) -> bool {
        // verify signature

        // verify viewchanges
        let viewchanges_valid = self.verify_viewchanges_in_newview(&newview.viewchanges);
        // verify preprepares
        let new_view = newview.view;
        let preprepares_valid = self.verify_preprepares_in_newview(
            &newview.viewchanges,
            &newview.preprepares,
            new_view,
        );

        viewchanges_valid && preprepares_valid
    }

    // Get the maximum sequence number based on the viewchange messages
    pub fn get_max_sequence_number_by_viewchanges(&self, viewchanges: &[ViewChange]) -> u64 {
        let mut max = 0 as u64;
        for viewchange in viewchanges {
            let mut prepare_messages_iter = viewchange.proof_messages.prepares.iter();
            loop {
                match prepare_messages_iter.next() {
                    Some(MessageType::Prepare(prepare)) if prepare.number > max => {
                        max = prepare.number
                    }
                    None => {
                        break;
                    }
                    _ => {}
                }
            }
        }

        max
    }

    // Verify the validity of viewchange messages in the NEWVIEW message
    pub fn verify_viewchanges_in_newview(&self, viewchanges: &[ViewChange]) -> bool {
        let mut viewchange_msg_from_ids: HashSet<String> = HashSet::new();
        viewchanges.iter().for_each(|v| {
            if viewchange_msg_from_ids.contains(&v.from_peer_id) && self.verify_viewchange(v) {
                viewchange_msg_from_ids.insert(v.from_peer_id.to_string());
            }
        });

        viewchange_msg_from_ids.len() as u64 > 2 * self.state.fault_tolerance_count
    }

    // Verify the validity of preprepare messages in the NEWVIEW message.
    // It verifies the correctness of preprepare messages
    // by performing a computation similar to the one used
    // by the primary to create preprepare messages.
    pub fn verify_preprepares_in_newview(
        &self,
        viewchanges: &[ViewChange],
        preprepares: &[PrePrepare],
        new_view: u64,
    ) -> bool {
        let min = viewchanges.first().unwrap().latest_stable_checkpoint;
        let max = self.get_max_sequence_number_by_viewchanges(viewchanges);

        let mut preprepare_map: HashMap<u64, PrePrepare> = HashMap::new();
        for viewchange in viewchanges {
            let mut preprepare_message_iter = viewchange.proof_messages.preprepares.iter();
            loop {
                match preprepare_message_iter.next() {
                    Some(MessageType::PrePrepare(preprepare))
                        if !preprepare_map.contains_key(&preprepare.number) =>
                    {
                        preprepare_map.insert(preprepare.number, preprepare.clone());
                    }
                    None => {
                        break;
                    }
                    _ => {}
                }
            }
        }

        let recreated_preprepares = ((min + 1)..(max + 1))
            .into_iter()
            .map(|i| {
                if let Some(preprepare) = preprepare_map.get(&i) {
                    preprepare.clone()
                } else {
                    PrePrepare {
                        view: new_view,
                        number: i,
                        m_hash: String::from("NULL"),
                        m: vec![],
                        signature: String::from(""),
                        from_peer_id: String::from(""),
                    }
                }
            })
            .collect::<Vec<PrePrepare>>();

        recreated_preprepares
            .iter()
            .zip(preprepares.iter())
            .all(|(p1, p2)| *p1 == *p2)
    }

    // Verify the validity of a viewchange
    pub fn verify_viewchange(&self, viewchange: &ViewChange) -> bool {
        // verify viewchange signature

        // verify every checkpoint
        let mut checkpoint_msg_from_ids: HashSet<String> = HashSet::new();
        viewchange
            .latest_stable_checkpoint_messages
            .iter()
            .filter(|c| {
                c.current_max_number == self.state.stable_checkpoint.0
                    && c.checkpoint_state_digest
                        .eq(&self.state.stable_checkpoint.1)
            })
            .for_each(|c| {
                // Avoid the same id and verify checkpoint signature
                if checkpoint_msg_from_ids.contains(&c.from_peer_id) && true {
                    checkpoint_msg_from_ids.insert(c.from_peer_id.clone());
                }
            });

        checkpoint_msg_from_ids.len() as u64 > 2 * self.state.fault_tolerance_count
    }
}
