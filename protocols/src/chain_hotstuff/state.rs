use std::sync::Arc;
use components::message::Request;
use tokio::sync::Mutex;

use super::message::QC;

pub struct State {
    pub view: u64,
    pub high_qc: Option<QC>,
    pub generic_qc: Option<QC>,
    pub locked_qc: Option<QC>,
    pub current_leader: Vec<u8>,
    pub next_leader: Vec<u8>,
    pub node_count: u64,
    pub fault_tolerance_count: u64,
    pub tf: u64,
    pub current_view_timeout: u64,
    pub current_request: Request,
}



impl State {
    pub fn new() -> Self {
        Self {
            view: 0,
            current_leader: vec![],
            next_leader: vec![],
            node_count: 4,
            fault_tolerance_count: 1,
            // mode: Arc::new(Mutex::new(Mode::Init)),
            high_qc: None,
            generic_qc: None,
            locked_qc: None,
            tf: 10,
            current_view_timeout: 10,
            current_request: Request {
                cmd: "None".to_string(),
            },
        }
    }
}
