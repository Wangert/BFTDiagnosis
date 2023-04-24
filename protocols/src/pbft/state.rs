use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use utils::coder::get_hash_str;
use components::message::Request;
use super::timer::Timeout;

pub struct State {
    pub view: u64,
    pub current_sequence_number: u64,
    //pub low_water: u64,
    pub primary: Vec<u8>,
    pub is_primary: bool,
    pub node_count: u64,
    pub fault_tolerance_count: u64,
    // Normal and abnormal mode in the consensus process
    pub mode: Arc<Mutex<Mode>>,
    pub stable_checkpoint: StableCheckpoint,
    pub current_commited_request_count: u64,
    pub checkpoint_state: String,
    // Client node's state
    pub client_state: ClientState,
    // Reach the prepared state timeout
    pub prepared_timeout: Timeout,
    // Reach the commited state timeout
    pub commited_timeout: Timeout,
    pub current_request: Request,
    pub timeout_flag: bool,
}

pub struct ControllerState {
    pub client_state: ClientState,
    pub fault_tolerance_count: u64,
}

pub struct StableCheckpoint(pub u64, pub String);

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    // The consensus node is normally in the consensus process
    Normal,
    // View change process
    Abnormal,
}

pub enum ClientState {
    NotRequest,
    Waiting,
    Replied,
}

pub enum PhaseState {
    NotRequest,
    Init,
    Prepared,
    Commited,
    Replied,
}

impl State {
    pub fn new(timeout_duration: Duration) -> State {
        let initial_checkpoint_state = [0 as u8; 64];
        let checkpoint_state = String::from_utf8_lossy(&initial_checkpoint_state).to_string();
        State {
            view: 0,
            current_sequence_number: 0,
            //low_water: 0,
            primary: Vec::new(),
            is_primary: false,
            node_count: 9,
            fault_tolerance_count: 1,
            mode: Arc::new(Mutex::new(Mode::Normal)),
            stable_checkpoint: StableCheckpoint(0, String::from("")),
            current_commited_request_count: 0,
            checkpoint_state,
            client_state: ClientState::NotRequest,
            prepared_timeout: Timeout::new(timeout_duration),
            commited_timeout: Timeout::new(timeout_duration),
            current_request: Request {
                cmd: "None".to_string(),
            },
            timeout_flag: false,
        }
    }

    pub fn update_checkpoint_state(&mut self, commited_request_hash: &str) {
        let old_checkpoint_state = self.checkpoint_state.clone();
        let old_checkpoint_state_u8s = old_checkpoint_state.as_bytes();
        let commited_request_hash_u8s = commited_request_hash.as_bytes();
        let new_checkpoint_state_vec: Vec<u8> = old_checkpoint_state_u8s
            .iter()
            .zip(commited_request_hash_u8s.iter())
            .map(|(a, b)| a ^ b)
            .collect();
        let new_checkpoint_state = get_hash_str(&new_checkpoint_state_vec);
        self.checkpoint_state = new_checkpoint_state;
    }

    // pub fn commited_timeout_tick_start(&mut self, notify: Arc<Notify>) {
    //     if let TimeoutState::Active = self.commited_timeout.state {
    //         let duration = self.commited_timeout.duration;
    //         let mut mode = self.mode.clone();
    //         let viewchange_closure = move || {
    //             let mut locked_mode = block_on(mode.lock());
    //             *locked_mode = Mode::Abnormal;
    //         };
    //         tokio::spawn(timeout_tick(duration, notify, viewchange_closure));
    //     }
    // }
}

impl ControllerState {
    pub fn new() -> Self {
        Self {
            client_state: ClientState::NotRequest,
            fault_tolerance_count: 1,
        }
    }
}


