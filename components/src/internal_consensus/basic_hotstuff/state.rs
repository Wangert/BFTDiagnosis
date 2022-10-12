use std::sync::Arc;
use tokio::sync::Mutex;

use crate::message::Request;

use super::message::QC;

pub struct State {
    pub view: u64,
    pub high_qc: Option<QC>,
    pub prepare_qc: Option<QC>,
    pub locked_qc: Option<QC>,
    pub commit_qc: Option<QC>,
    pub primary: Vec<u8>,
    pub node_count: u64,
    pub fault_tolerance_count: u64,
    pub tf: u64,
    pub current_view_timeout: u64,
    pub current_request: Request,
    pub current_id: Vec<u8>,
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
            high_qc: None,
            prepare_qc: None,
            locked_qc: None,
            commit_qc: None,
            tf: 10,
            current_view_timeout: 10,
            
            current_id: Vec::new(),
            current_request: Request {
                cmd: "None".to_string(),
            },
        }
    }
}