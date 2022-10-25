use super::ba_broadcast::Message as BaMessage;
use super::bool_set::{self, BoolSet};
use super::AbaMessage;
use super::Message;
use super::SigMessage;
use crate::honeybadger::threshold_sign::ThresholdSign;
use rand::distributions::Open01;
use serde::ser::SerializeMap;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use threshold_crypto::{
    PublicKey, PublicKeySet, PublicKeyShare, SecretKeySet, SecretKeyShare, Signature,
    SignatureShare,
};
use utils::coder::{self, deserialize_for_bytes, serialize_into_bytes};

#[derive(Debug)]
pub enum CoinState {
    //本轮的value已经确定或者coin已经终止
    Decided(bool),
    //coin value现在还未知
    InProgress(Box<ThresholdSign>),
}

impl CoinState {
    fn value(&self) -> Option<bool> {
        match self {
            CoinState::Decided(value) => Some(*value),
            CoinState::InProgress(_) => None,
        }
    }
}

impl From<bool> for CoinState {
    fn from(value: bool) -> CoinState {
        CoinState::Decided(value)
    }
}

/// 在BA协议的某一个epoch中从其他节点收到的信息,为之后的视图切换提供帮助
#[derive(Debug)]
struct ReceivedMessages {
    /// 收到的Bval消息
    bval: BoolSet,
    /// 收到的Aux消息
    aux: BoolSet,
    /// 收到的Conf消息
    conf: Option<BoolSet>,
    /// 收到的Term消息
    term: Option<bool>,
    /// 收到的coin消息
    coin: Option<SignatureShare>,
}

impl ReceivedMessages {
    fn new() -> Self {
        ReceivedMessages {
            bval: bool_set::NONE,
            aux: bool_set::NONE,
            conf: None,
            term: None,
            coin: None,
        }
    }

    pub fn insert(&mut self, message: AbaMessage) {
        match message {
            AbaMessage::BaBroadcast(id,msg) => match msg {
                super::ba_broadcast::Message::BVAL(id,value) => {
                    if !self.bval.insert(value) {
                        return;
                    }
                }
                super::ba_broadcast::Message::AUX(id,value) => {
                    if !self.aux.insert(value) {
                        return;
                    }
                }
            },
            AbaMessage::Conf(id,msg) => {
                if self.conf.is_none() {
                    self.conf = Some(msg);
                } else {
                    return;
                }
            }
            AbaMessage::Term(id,msg) => {
                if self.term.is_none() {
                    self.term = Some(msg);
                } else {
                    return;
                }
            }
            AbaMessage::Coin(id,msg) => {
                if self.coin.is_none() {
                    self.coin = Some(msg.0);
                } else {
                    return;
                }
            }
            //AbaMessage::SignatureShare(msg) => todo!(),
            AbaMessage::Signature(id,msg) => todo!(),
        }
    }

    /// Creates message content from `ReceivedMessages`. That message content can then be handled.
    fn messages(self) -> Vec<AbaMessage> {
        let ReceivedMessages {
            bval,
            aux,
            conf,
            term,
            coin,
        } = self;
        let mut messages = Vec::new();
        for b in bval {
            messages.push(AbaMessage::BaBroadcast(String::new(),BaMessage::BVAL(String::new(),b)));
        }
        for b in aux {
            messages.push(AbaMessage::BaBroadcast(String::new(),BaMessage::AUX(String::new(),b)));
        }
        if let Some(bs) = conf {
            messages.push(AbaMessage::Conf(String::new(),bs));
        }
        if let Some(b) = term {
            messages.push(AbaMessage::Term(String::new(),b));
        }
        if let Some(ss) = coin {
            messages.push(AbaMessage::Coin(String::new(),Box::new(super::SigMessage(ss))));
        }
        messages
    }
}

//ABA的核心结构
#[derive(Debug)]
pub struct BinaryAgreement {
    //共识网络设置，储存节点信息以及门限签名的密钥
    netinfo: crate::honeybadger::network::Network,
    proposer_id:String,
    //防重放的session，可选择
    nonce: usize,
    //轮数
    epoch: usize,
    //最大的轮数
    max_epoch: usize,
    //广播实例
    pub ba_broadcast: super::ba_broadcast::BvBroadcast,
    //从BVAL广播中收到的values输出
    bv_output: BoolSet,
    //收到的Conf消息，在每个新的epoch开始时重置
    conf_received: HashMap<String, BoolSet>,
    //收到的term消息
    term_received: HashMap<bool, HashSet<String>>,
    //本轮中预估的value
    estimated: Option<bool>,
    //决定的value
    pub decision: Option<bool>,
    /// A cache for messages for future epochs that cannot be handled yet.
    incoming_queue: HashMap<usize, HashMap<String, ReceivedMessages>>,
    /// 第一个 2f+1 个 `Aux` 消息中的values`.
    conf_values: Option<BoolSet>,
    /// 该epoch中的coin的状态
    coin_state: CoinState,
    //存储节点收到的SignatureShare
    //sigshare_received: Vec<Box<SignatureShare>>
    //存储节点收到的完整的Signature
    signature: Option<Box<Signature>>,
}

impl BinaryAgreement {
    pub fn new(netinfo: crate::honeybadger::network::Network, nonce: usize,proposer_id:&str) -> Self {
        BinaryAgreement {
            netinfo: netinfo.clone(),
            nonce,
            epoch: 0,
            max_epoch: 1000,
            ba_broadcast: super::ba_broadcast::BvBroadcast::new(netinfo,0,proposer_id),
            conf_received: HashMap::new(),
            term_received: HashMap::new(),
            estimated: None,
            decision: None,
            incoming_queue: HashMap::new(),
            conf_values: None,
            coin_state: CoinState::Decided(true),
            bv_output: bool_set::NONE,
            signature: None,
            proposer_id: proposer_id.to_string(),
            //sigshare_received: Vec::new()
        }
    }

    //propose一个bool值
    pub async fn propose(&mut self, input: bool) {
        println!("ABA开始propose！");
        if !self.can_propose() {
            return;
        }
        println!("预估的value为:{}",input);
        self.estimated = Some(input);
        println!("开始广播bval");
        self.ba_broadcast.send_bval(input).await;
        //每次启动广播都要做状态检查
        self.handle_babroadcast_sync().await;
    }

    //收到广播的消息，根据epoch进行处理，过期则丢弃，太新则存入缓存，相等则进行处理
    pub async fn handle_message(&mut self, source: &str, msg: &Vec<u8>) -> Option<bool> {
        let msg = coder::deserialize_for_bytes(msg);
        let Message { epoch, content } = msg;
        println!("当前情况：收到的epoch：{},自己的epoch{}",epoch,self.epoch);
        if self.decision.is_some() || epoch < self.epoch {
            println!("消息以过期: 已收到一个更新epoch的消息.");
            None
        } else if epoch > self.epoch + self.max_epoch {
            println!("已超过最大epoch！");
            None
        } else if epoch > self.epoch {
            println!("收到一个更新的消息，进行缓存");
            let epoch_state = self.incoming_queue.entry(epoch).or_insert(HashMap::new());
            let received = epoch_state
                .entry(source.to_string())
                .or_insert(ReceivedMessages::new());
            received.insert(content);
            None
        } else {
            self.handle_message_content(source, content).await
        }
    }

    //处理传入的AbaMessage
    pub async fn handle_message_content(&mut self, source: &str, msg: AbaMessage) -> Option<bool>{
        println!("epoch检查通过，开始进行aba消息的具体逻辑处理");
        match msg {
            AbaMessage::BaBroadcast(id,msg) => self.handle_babroadcast(source, &msg).await,
            AbaMessage::Conf(id,value) => self.handle_conf(id.as_str(), value).await,
            //AbaMessage::Term(id,value) => self.handle_term(id.as_str(), value).await,
            AbaMessage::Term(id,value) => (),
            AbaMessage::Coin(id,value) => self.handle_coin(id.as_str(), value).await,
            //AbaMessage::SignatureShare(value) => self.handle_sigshare(source, value),
            AbaMessage::Signature(id,value) => self.handle_signature(id.as_str(), value),
        }
        self.decision
    }

    //节点收到signature消息
    pub fn handle_signature(&mut self, source: &str, msg: Box<Signature>) {
        if let Some(_) = &self.signature {
            return;
        }
        self.signature = Some(msg);
    }

    pub async fn handle_babroadcast(&mut self, source: &str, msg: &super::ba_broadcast::Message) {
        let serialized_msg = coder::serialize_into_bytes(&msg);
        self.ba_broadcast.handle_message(source, &serialized_msg).await;
        self.handle_babroadcast_sync().await;
    }

    //每次启动广播都要做状态检查,看是否收到了来自bval广播的output，如果收到了，根据coin的状态选择尝试更新视图或/决定还是发送conf信息
    pub async fn handle_babroadcast_sync(&mut self) {
        println!("开始进行广播状态检查！");
        let values = self.ba_broadcast.get_output();
        if(!values.contains(false) && !values.contains(true)){
            println!("未得到BVCAL广播的输出，返回！");
            return
        }
        if values != bool_set::NONE {
            self.bv_output = values.clone();
        }
        //如果之前已经发过了conf则退出
        if self.conf_values.is_some() {
            return; // The `Conf` round has already started.
        }
        
        match self.coin_state {
            CoinState::Decided(_) => {
                self.conf_values = Some(values);
                self.try_update_epoch().await
            }
            CoinState::InProgress(_) => {
                //发送conf消息（唯一一次发送）
                self.send_conf(values).await;
            }
        }
    }

    /// 广播Conf（values）信息
    async fn send_conf(&mut self, values: BoolSet) {
        if self.conf_values.is_some() {
            // 一个epoch中只运行广播一次Cong消息
            return;
        }
        // Conf阶段开始
        self.conf_values = Some(values.clone());
        let msg = AbaMessage::Conf(self.netinfo.id.clone(),values);
        let message = msg.with_epoch(self.epoch);
        let serialized_msg = coder::serialize_into_bytes(&message);
        self.ba_broadcast.broadcast(&serialized_msg).await;
    }

    //收到2f+1个Conf时，更新epoch或者决定输出
    pub async fn handle_conf(&mut self, source: &str, values: BoolSet) {
        self.conf_received.insert(source.to_string(), values);
        self.try_finish_conf_round().await;
    }

    //检查是否收到了2f+1个Conf消息，如果正确，激活coin
    pub async fn try_finish_conf_round(&mut self) {
        if self.conf_received.len() == 0 || self.conf_received.len() < self.netinfo.num_honest() {
            return;
        }
        //激活coin
        let msg: Option<super::SigMessage> = match self.coin_state {
            CoinState::Decided(_) => return, //coin已经被激活
            //根据自己的secretekeyshare和pk签名，广播签名 ,用自己的私钥分享对信息签名
            CoinState::InProgress(ref mut ts) => ts.sign(), //激活coin
        };
        //广播自己的SignatureShare
        let message = AbaMessage::Coin(self.netinfo.id.clone(),Box::new(msg.unwrap()));
        let msg = message.with_epoch(self.epoch);
        let serialized_msg = coder::serialize_into_bytes(&msg);
        self.ba_broadcast.broadcast(&serialized_msg).await;

        //更新coin状态
        self.coin_sync().await
    }

    

    //coin的检查函数，如果有输出则以完整签名的奇偶性作为coin value
    pub async fn coin_sync(&mut self) {
       // let epoch = self.epoch;
        if let Some(value) = &self.signature {
            
            self.coin_state = value.parity().into();
            self.try_update_epoch().await;
        } else {
        }
    }

    
    //收到SignatreShare并校验，存在本地，
    //如果收到f+1个SignatureShare得到完整的签名，则发送Signature消息，更新coin状态看是否可以decide活切换epoch
    pub async fn handle_coin(&mut self, source: &str, value: Box<SigMessage>) {
        let sig = match self.coin_state {
            CoinState::Decided(_) => None, // Coin value is already decided.

            // 收到SignatreShare并校验，存在本地，如果收到f+1个SignatureShare，则返回完整的签名
            CoinState::InProgress(ref mut ts) => {
                //验证收到的ss是否正确
                //聚合签名，如果成功则输出
                ts.handle_message(source, &value)
            }
        };
        //如果收到了signature，则发送Signature（）消息
        if let Some(v) = sig {
            let msg = AbaMessage::Signature(self.netinfo.id.clone(),Box::new(v));
            let message = msg.with_epoch(self.epoch);
            let seriliazed_msg = coder::serialize_into_bytes(&message);
            self.ba_broadcast.broadcast(&seriliazed_msg).await;
        } else {
        }
        self.coin_sync().await
        //coin更新状态，如果已经收到了完整的Signatue则此时已经有了output，其值为得到的完整Signatue
    }

    //判断是否可以propose一个value
    pub fn can_propose(&self) -> bool {
        self.epoch == 0 && self.estimated.is_none()
    }

    
    async fn try_update_epoch(&mut self) {
        if self.decision.is_some() {
            // Avoid an infinite regression without making a Binary Agreement step.
            return;
        }
        let coin = match self.coin_state.value() {
            None => return, // Still waiting for coin value.
            Some(coin) => coin,
        };
        let def_bin_value = match self.conf_values {
            None => return, // Still waiting for conf value.
            Some(ref values) => values.definite(),
        };

        if Some(coin) == def_bin_value {
            self.decide(coin).await
        } else {
            self.update_epoch(def_bin_value.unwrap_or(coin)).await
        }
    }

    //决定一个value并且广播Term消息
    pub async fn decide(&mut self, b: bool) {
        if self.decision.is_some() {
            return;
        }
        // Output the Binary Agreement value.
        self.decision = Some(b);
        // println!("decision: {}", b);
        println!("**********************************************************************");
        println!("针对节点：{}的proposer的共识以达成，结果为：{}",self.proposer_id,b);
        println!("**********************************************************************");
        //广播Term消息
        // let msg = AbaMessage::Term(self.netinfo.id.clone(),b);
        // let message = msg.with_epoch(self.epoch);
        // let serialized_msg = coder::serialize_into_bytes(&message);
        // self.ba_broadcast.broadcast(&serialized_msg).await;

        // let msg = crate::honeybadger::subset::message::MessageContent::Reply(self.proposer_id.clone(), true);
        // let data = msg.with(self.proposer_id.as_str());
        // let val = coder::serialize_into_bytes(&data);
        // self.ba_broadcast.broadcast(&val).await;
        
    }

    pub fn get_output(&self) -> Option<bool> {
        self.decision
    }

    /// 更新epoch时调用此函数得到一个新的coin_state，有1/3的几率得到一个in_process的coin_state
    fn coin_state(&self) -> CoinState {
        match self.epoch % 3 {
            0 => CoinState::Decided(true),
            1 => CoinState::Decided(false),
            _ => {
                //let coin_id = coder::serialize_into_bytes(&(&self.nonce, self.epoch));
                let mut ts = ThresholdSign::new(self.netinfo.clone());
                //ts.set_document(coin_id).map_err(Error::InvokeCoin)?;

                CoinState::InProgress(Box::new(ts))
            }
        }
    }

    //递增epoch，设置新的估计值并处理排队的消息。
    pub async fn update_epoch(&mut self, b: bool) {
        self.ba_broadcast.clear(&self.term_received);
        self.conf_received.clear();

        for i in self.term_received.iter() {
            for p in i.1 {
                let mut bset = match self.conf_received.get(p) {
                    Some(value) => value.clone(),
                    None => bool_set::NONE.clone(),
                };
                bset.insert(i.0.to_owned());
                self.conf_received.entry(p.to_string()).or_insert(bset);
            }
        }

        self.conf_values = None;
        self.epoch += 1;
        self.coin_state = self.coin_state();
        println!("epoch started, {} terminated", self.conf_received.len(),);

        self.estimated = Some(b);
        self.ba_broadcast.send_bval(b).await;
        //self.handle_babroadcast_sync().await;
        let epoch = self.epoch;
        let epoch_state = self.incoming_queue.remove(&epoch).into_iter().flatten();
        for (sender_id, received) in epoch_state {
            for m in received.messages() {
                self.handle_message_content(&sender_id, m);
                if self.decision.is_some() || self.epoch > epoch {
                    return;
                }
            }
        }
    }

    
}
