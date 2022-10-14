use std::collections::{BTreeMap, HashMap};

use utils::coder;

use crate::honeybadger::{network::Network, reliable_broadcast::broadcast::PeerSet};
use tokio::sync::mpsc::{self, Receiver, Sender};
use super::{
    message::{self, Message},
    state::State,
};

#[derive(Debug)]
pub struct Subset {
    //节点的信息以及密钥
    pub netinfo: Network,
    //nonce 猜测此处指的是对于每一个ABA的编号，存在一个sunset，因为此处的decided为bool值
    nonce: usize,
    pub result:HashMap<String,bool>,
    pub tx_result:HashMap<String,String>,
    /// 一个映射
    pub states: HashMap<String, State>,
    //当前是否有输出
    decided: bool,
    pub rbc_msg_tx: Sender<Vec<u8>>,
    pub rbc_msg_rx: Receiver<Vec<u8>>,
    pub aba_msg_tx: Sender<Vec<u8>>,
    pub aba_msg_rx: Receiver<Vec<u8>>,
}

impl Subset {
    pub fn new(netinfo: Network, nonce: usize) -> Self {
        let mut states: HashMap<String, State> = HashMap::new();
        let mut result:HashMap<String, bool> = HashMap::new();
        let mut tx_result:HashMap<String,String> = HashMap::new();
        let (rbc_msg_tx, rbc_msg_rx) = mpsc::channel::<Vec<u8>>(10);
        let (aba_msg_tx, aba_msg_rx) = mpsc::channel::<Vec<u8>>(10);
        for i in netinfo.connected_nodes.values() {
            states.insert(
                i.to_string(),
                State::new(netinfo.clone(), nonce, rbc_msg_tx.clone(), aba_msg_tx.clone(),i.to_string()),
            );
            result.insert(i.to_string(), false);
            tx_result.insert(i.to_string(), "".to_string());
        }
        Subset {
            result,
            tx_result,
            netinfo,
            nonce,
            states: states,
            decided: false,
            rbc_msg_rx,
            rbc_msg_tx,
            aba_msg_rx,
            aba_msg_tx
        }
    }

    //为当前的ACS propose一个value
    pub async fn propose(&mut self, value: Vec<u8>,source:&str) {
        // println!("{}Subset propose: {:?}", self.nonce, value);
        // println!("This is {} , state is {:?}",self.netinfo.id,self.states.keys());
            self
            .states
            .get_mut(self.netinfo.id.as_str())
            .unwrap()
            .propose(value,self.netinfo.clone(),source).await;
    }
    /// 处理收到的Message信息.
    pub async fn handle_message(&mut self, source: &str, message: &Vec<u8>) {
        let msg:super::message::Message = coder::deserialize_for_bytes(message);
        match msg.content {
            super::message::MessageContent::Request(input) => self.propose(input.data,source).await,
            
            super::message::MessageContent::Reply(proposer_id,confirm) => {
                
            }
            _ => {
                //println!("收到gossip的消息，进入state,proposer为：{:?}",msg.proposer_id);
                
                self.states
                .get_mut(msg.proposer_id.as_str())
                .unwrap()
                .handle_message(source, msg.content).await;
            }
        }
        // for (i,j) in &self.states {
        //     match j {
        //         State::Complete(true) => {
        //             self.result.insert(i.to_string(), true);
        //         },
        //         State::GetValue(value, _) => {
        //             self.tx_result.insert(i.to_string(), String::from_utf8(value.to_vec().clone()).unwrap());
        //         }
                
        //         _ => ()
        //     }
        // }
        // println!("************************************************************");
        
        // println!("共识过程结束");
        // println!("************************************************************");
        // println!("");
        // println!("************************************************************");
        // println!("最终的共识结果为：{:?}",self.result);
        // println!("************************************************************");
        // println!("");
        // println!("************************************************************");
        // println!("得到的各个节点的交易分别为：{:?}",self.tx_result);
        // println!("************************************************************");

    }

    /// Returns the number of validators from which we have already received a proposal.
    pub fn received_proposals(&self) -> usize {
        let received = |state: &&State| state.received();
        self.states.values().filter(received).count()
    }

    /// Returns the number of Binary Agreement instances that have decided "yes".
    fn count_accepted(&self) -> usize {
        let accepted = |state: &&State| state.accepted();
        self.states.values().filter(accepted).count()
    }

    /// Checks the voting and termination conditions: If enough proposals have been accepted, votes
    /// "no" for the remaining ones. If all proposals have been decided, outputs `Done`.
    fn try_output(&mut self) {
        if self.decided || self.count_accepted() < self.netinfo.num_honest() {
            
        }
        //收到N-f个1后向其他的aba输入0
        if self.count_accepted() == self.netinfo.num_honest() {
            for (proposer_id, state) in &mut self.states {
                
            }
        }
        if self.states.values().all(State::complete) {
            self.decided = true;
            println!("ACS过程已结束")
        }
    }

    
}
