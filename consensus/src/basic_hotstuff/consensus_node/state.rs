use std::sync::Arc;
use tokio::sync::Mutex;

use super::message::QC;

pub struct State {
    pub view: u64,
    pub high_qc: Option<QC>,
    pub prepare_qc: Option<QC>,
    pub locked_qc: Option<QC>,
    pub commit_qc: Option<QC>,
    pub primary: Vec<u8>,
    // pub is_primary: Arc<Mutex<bool>>,
    // pub hava_request: Arc<Mutex<bool>>,
    pub node_count: u64,
    pub fault_tolerance_count: u64,
    pub tf: u64,
    pub current_view_timeout: u64,

    pub mode: Arc<Mutex<Mode>>,
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Done(u64),
    Do(u64),
    NotIsLeader(u64),
    Init,
}

impl State {
    pub fn new() -> Self {
        Self {
            view: 0,
            primary: vec![],
            // is_primary: Arc::new(Mutex::new(false)),
            // hava_request: Arc::new(Mutex::new(false)),
            node_count: 4,
            fault_tolerance_count: 1,
            mode: Arc::new(Mutex::new(Mode::Init)),
            high_qc: None,
            prepare_qc: None,
            locked_qc: None,
            commit_qc: None,
            tf: 10,
            current_view_timeout: 10,
        }
    }
}

#[cfg(test)]
mod state_test {
    #[test]
    fn checkpoint_state_works() {}
}
