use components::{
    behaviour::{PhaseState, SendType},
    message::Request,
};
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::Mutex;

use super::{common::get_block_hash, message::QC};

pub struct State {
    pub view: u64,
    pub high_qc: Option<QC>,

    pub generic_qc: Option<QC>,
    pub locked_qc: Option<QC>,
    pub commit_qc: Option<QC>,
    pub temp_qc: Option<QC>,
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
            commit_qc: None,
            temp_qc: None,
            tf: 5,
            current_view_timeout: 5,
            current_request: Request {
                cmd: "None".to_string(),
            },
        }
    }

    pub fn update_state_and_output(
        &mut self,
        generic_qc: Option<QC>,
        send_query: VecDeque<SendType>,
    ) -> PhaseState {
        println!("调用update函数");
        let (b_2, new_qc) = if let Some(qc) = &generic_qc {
            (qc.block.clone(), qc.clone())
        } else {
            return PhaseState::Complex(None, send_query);
        };

        if let Some(generic_qc) = self.generic_qc.clone() {
            println!("有generic_qc");
            // 如果有generic_qc
            let b_2_hash = get_block_hash(&self.generic_qc.clone().unwrap().block);
            if b_2.parent_hash == b_2_hash {
                if let Some(locked_qc) = self.locked_qc.clone() {
                    println!("有locked_qc");
                    // 如果有locked_qc
                    let b_3 = locked_qc.block.clone();
                    let b3_hash = get_block_hash(&b_3);
                    if true {
                        if let Some(commit_qc) = self.commit_qc.clone() {
                            println!("有commit_qc");

                            self.commit_qc = Some(locked_qc);
                            self.locked_qc = Some(generic_qc);
                            self.generic_qc = Some(new_qc);
                            // 输出block
                        } else {
                            // 没有commit_qc
                            self.commit_qc = Some(locked_qc);
                            self.locked_qc = Some(generic_qc);
                            self.generic_qc = Some(new_qc);
                            println!("初始化commit_qc");
                        }
                        let b_4 = self.commit_qc.clone().unwrap().block;
                        println!("");
                        println!("");
                        println!("+++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
                        println!("+  Execute new commands, current command is:");
                        println!("+  {:?}", b_4.cmd);
                        println!("+++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
                        println!("");
                        println!("");
                        let request = Request {
                            cmd: b_4.clone().cmd,
                        };

                        // self.taken_requests
                        //     .remove(&coder::serialize_into_bytes(&request));

                        return PhaseState::Complex(Some(request), send_query);
                    } else {
                        println!("验证出错");
                        return PhaseState::Complex(None, send_query);
                    }
                } else {
                    // 如果没有locked_qc
                    println!("初始化locked_qc");
                    self.locked_qc = Some(generic_qc);
                    self.generic_qc = Some(new_qc);
                    return PhaseState::Complex(None, send_query);
                }
            } else {
                println!("验证失败");
                return PhaseState::Complex(None, send_query);
            }
        } else {
            // 没有generic_qc
            println!("初始化generic_qc");
            self.generic_qc = Some(new_qc);
            return PhaseState::Complex(None, send_query);
        }
    }

    pub fn update_state(&mut self, generic_qc: Option<QC>) {
        let (b_2, new_qc) = if let Some(qc) = &generic_qc.clone() {
            (qc.block.clone(), qc.clone())
        } else {
            return;
        };
        if let Some(generic_qc) = self.generic_qc.clone() {
            // 如果有generic_qc
            if let Some(locked_qc) = self.locked_qc.clone() {
                // 如果有locked_qc
                if let Some(commit_qc) = self.commit_qc.clone() {
                    // 如果有commit_qc
                    self.commit_qc = Some(locked_qc);
                    self.locked_qc = Some(generic_qc);
                    self.generic_qc = Some(new_qc);
                    return;
                } else {
                    // 没有commit_qc
                    self.commit_qc = Some(locked_qc);
                    self.locked_qc = Some(generic_qc);
                    self.generic_qc = Some(new_qc);
                    return;
                }
            } else {
                // 如果没有locked_qc
                self.locked_qc = Some(generic_qc);
                self.generic_qc = Some(new_qc);
                return;
            }
        } else {
            // 没有generic_qc
            self.generic_qc = Some(new_qc);
            return;
        }
    }
}
