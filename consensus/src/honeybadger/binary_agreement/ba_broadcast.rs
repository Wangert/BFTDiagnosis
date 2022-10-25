use super::bool_set::{self, BoolSet};
use crate::honeybadger::network;
use crate::honeybadger::subset;
use crate::honeybadger::subset::message::Message as SubsetMessage;
use crate::honeybadger::subset::message::MessageContent as SubsetMessageContent;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc::{self, Receiver, Sender};
use utils::coder::{self, get_hash_str};

use super::AbaMessage;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Message {
    BVAL(String, bool),
    AUX(String, bool),
}

#[derive(Debug)]
pub struct BvBroadcast {
    //HoneyBadger网络中的节点，密钥信息
    network: network::Network,
    proposer_id:String,
    pub epoch: usize,
    //存放已经收到2f+1个Bval(b)信息中包含的b
    bin_values: BoolSet,
    //表示哪些节点向当前节点发送了Bval(b)
    pub bval_from: HashMap<bool, HashSet<String>>,
    //表示当前节点已发送了哪些b
    bval_sent: BoolSet,
    //表示哪些节点向当前节点发送了Aux(b)
    aux_from: HashMap<bool, HashSet<String>>,
    //是否终止
    terminated: bool,
    output: BoolSet,
    //发送和接收信息的管道
    pub msg_sender: Sender<Vec<u8>>,
    pub msg_receiver: Receiver<Vec<u8>>,
}

impl BvBroadcast {
    pub fn new(network: network::Network, epoch: usize,proposer_id:&str) -> BvBroadcast {
        let (msg_tx, msg_rx) = mpsc::channel::<Vec<u8>>(10);
        let mut init_map: HashMap<bool, HashSet<String>> = HashMap::new();
        init_map.insert(true, HashSet::new());
        init_map.insert(false, HashSet::new());
        let mut aux_init_map:HashMap<bool,HashSet<String>> = HashMap::new();
        aux_init_map.insert(true, HashSet::new());
        aux_init_map.insert(false, HashSet::new());
        BvBroadcast {
            network,
            epoch,
            bin_values: bool_set::NONE,
            bval_from: init_map,
            bval_sent: bool_set::NONE,
            aux_from: aux_init_map,
            terminated: false,
            msg_sender: msg_tx,
            msg_receiver: msg_rx,
            output: bool_set::NONE,
            proposer_id: proposer_id.to_string(),
        }
    }

    pub fn clear(&mut self, init: &HashMap<bool, HashSet<String>>) {
        self.bin_values = bool_set::NONE;
        self.bval_from = init.clone();
        self.bval_sent = bool_set::NONE;
        self.aux_from = init.clone();
        self.terminated = false;
    }

    pub async fn handle_message(&mut self, source: &str, message: &Vec<u8>) {
        let msg = coder::deserialize_for_bytes(message);
        match msg {
            Message::BVAL(id, b) =>{
                self.handle_bval(source, b).await;
            } ,
            Message::AUX(id, b) => self.handle_aux(source, b),
            _ => (),
        }
    }

    //收到BVAL(b)后的处理逻辑
    pub async fn handle_bval(&mut self, source: &str, b: bool) {
        println!("接受到新的BVAL消息！开始对收到的BVAL消息进行处理！");
        //判断是否已经受到过该节点的bval消息
        if self.bval_from.get(&b).unwrap().contains(source) {
            return;
        }

        // let mut check = false;
        // for i in self.bval_from.keys() {
        //     if i.to_owned() == b && self.bval_from.get(&b).unwrap().contains(source) {
        //         check = true;
        //     }
        // }
        // if(check) {
        //     return
        // }
        self.bval_from
            .get_mut(&b)
            .unwrap()
            .insert(source.to_string());
        let count_bval = self.bval_from.get(&b).unwrap().len();
        println!("已收到{}个BVAL消息", count_bval.clone());
        //如果收到了2f+1个bval，就发送aux信息
        if count_bval == 2 * self.network.get_fault_num() + 1 {
            println!("已收到2f+1个BVAL消息，准备发送AUX消息!");
            self.bin_values.insert(b);
            if self.bin_values != bool_set::BOTH {
                self.send_aux(b).await;
            } else {
                self.try_output();
            }
        }

        //如果收到了f+1个节点的bval信息，就发bval广播
        if count_bval == self.network.get_fault_num() + 1 {
            println!("已收到f+1个BVAL消息，准备发送BVAL消息!");
            self.send_bval(b).await;
        }
    }

    pub async fn send_aux(&mut self, value: bool) {
        let msg = Message::AUX(self.network.id.clone(), value);
        let message = AbaMessage::BaBroadcast(self.network.id.clone(), msg);
        let data = message.with_epoch(self.epoch);
        let subset_content = SubsetMessageContent::Agreement(self.network.id.clone(), data);
        let subset_msg = subset_content.with(self.proposer_id.as_str());
        let serialized_msg = coder::serialize_into_bytes(&subset_msg);
        self.broadcast(&serialized_msg).await;
    }

    pub async fn send_bval(&mut self, value: bool) {
        let msg = Message::BVAL(self.network.id.clone(), value);
        let message = AbaMessage::BaBroadcast(self.network.id.clone(), msg);
        let data = message.with_epoch(self.epoch);
        let subset_content = SubsetMessageContent::Agreement(self.network.id.clone(), data);
        let subset_msg = subset_content.with(self.proposer_id.as_str());
        let serialized_msg = coder::serialize_into_bytes(&subset_msg);
        self.broadcast(&serialized_msg).await;
    }

    //收到AUX(b)之后的处理逻辑
    pub fn handle_aux(&mut self, source: &str, b: bool) {
        println!("收到AUX消息，开始对收到的AUX消息处理！");
        if self.aux_from.get(&b).unwrap().contains(source) {
            return;
        }
        self.aux_from
            .get_mut(&b)
            .unwrap()
            .insert(source.to_string());
        self.try_output();
    }

    //检查是否有2f+1个Aux消息，并且其携带的value在bin_values中，最后输出
    pub fn try_output(&mut self) {
        if self.terminated || self.bin_values == bool_set::NONE {
            return;
        }
        let (count, values) = self.count_aux();
        if count < self.network.num_honest() {

            println!("当前共收到{}个AUX消息",count);
            return;
        }
        println!("收到2f+1个AUX消息，输出！");
        self.terminated = true;
        //输出output
        self.output = values;
    }

    pub fn get_output(&self) -> BoolSet {
        if self.output != bool_set::NONE {
            self.output.clone()
        } else {
            bool_set::NONE
        }
    }
    pub fn count_aux(&mut self) -> (usize, BoolSet) {
        let mut values = bool_set::NONE;
        let mut count = 0;

        for b in self.bin_values {
            if !self.aux_from.get(&b).unwrap().is_empty() {
                values.insert(b);
                count += self.aux_from.get(&b).unwrap().len();
            }
        }
        (count, values)
    }

    pub async fn broadcast(&self, msg: &[u8]) {
        self.msg_sender.send(msg.to_vec()).await.unwrap();
    }
}
