// use super::error::{Error, FaultKind};
use byteorder::{BigEndian, ByteOrder};
use chrono::offset::LocalResult;
use chrono::prelude::*;
use hex_fmt::{HexFmt, HexList};
use log::{debug, warn};
use rand::Rng;
use reed_solomon_erasure as rse;
use reed_solomon_erasure::{galois_8::Field as Field8, ReedSolomon};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Arc;
use std::{fmt, result};
use tokio::sync::mpsc::{self, Receiver, Sender};
use super::message::{self, Echo, Message, Ready};
use utils::coder::{self, get_hash_str};
use super::merkle::{Digest, MerkleTree, Proof};
use super::message::HexProof;
//use super::{Error, FaultKind, Message, Result};
//use crate::fault_log::Fault;
//use crate::{ConsensusProtocol, NodeIdT, Target, ValidatorSet};
use libp2p::PeerId;
type RseResult<T> = result::Result<T, rse::Error>;
use crate::honeybadger::subset::message::Message as SubsetMessage;
use crate::honeybadger::subset::message::MessageContent as SubsetMessageContent;

#[derive(Debug, Clone)]
pub struct PeerSet {
    pub marks: HashMap<String, usize>,
}

impl PeerSet {
    pub fn new() -> PeerSet {
        PeerSet {
            marks: HashMap::new(),
        }
    }

    pub fn insert(&mut self,id:String) {
        let num = self.num();
        self.marks.entry(id).or_insert(num);
    }
    
    // pub fn change_value(&mut self, peer_id:&str,num:usize) {
    //     self.marks.entry(peer_id.to_string()).and_modify(num);
    // }

    pub fn contains(&self, id: &str) -> bool {
        self.marks.contains_key(id)
    }

    pub fn index(&self, id: &str) -> Option<usize> {
        self.marks.get(id).cloned()
    }

    pub fn num(&self) -> usize {
        self.marks.len()
    }
}

#[derive(Debug)]
pub struct Broadcast {
    my_id: String,
    netinfo:crate::honeybadger::network::Network,
    pub msg_tx: Sender<Vec<u8>>,
    pub msg_rx: Receiver<Vec<u8>>,
    pub peer_set: PeerSet,
    coding: Coding,
    //ready_sent: bool,
    pub ready_sent: HashMap<String,bool>,
    can_decode: HashSet<Digest>,
    decided: bool,
    output:Vec<u8>,
    fault_estimate: usize,
    echos: HashMap<String, EchoContent>,
    pub echo_set: HashMap<String,HashSet<Proof<Vec<u8>>>> ,
    readys: HashMap<String, Vec<u8>>,
}

impl Broadcast {
    pub fn new(myid: &str, netinfo:crate::honeybadger::network::Network) -> Self {
        let (msg_tx, msg_rx) = mpsc::channel::<Vec<u8>>(10);
        //parity_shard_num = 2*f , f为预估的拜占庭节点个数
        let parity_shard_num = 2;
        //data_shard_num = N-parity_shard_num
        let data_shard_num = 2;
        let coding = Coding::new(data_shard_num, parity_shard_num)
            // .map_err(|_| )
            .expect("纠删码配置初始化失败！");
        let fault_estimate = netinfo.get_fault_num();
        let mut init_map = HashMap::new();
        init_map.entry(myid.to_string()).or_insert(HashSet::new());
        for i in netinfo.connected_nodes.values() {
            init_map.entry(i.to_string()).or_insert(HashSet::new());
        }
        let mut init_ready_sent = HashMap::new();
        init_ready_sent.entry(myid.to_string()).or_insert(false);
        for i in netinfo.connected_nodes.values(){
            init_ready_sent.entry(i.to_string()).or_insert(false);
        }
        let peer_set = PeerSet::new();
        Broadcast {
            my_id: myid.to_string(),
            netinfo,
            msg_tx,
            msg_rx,
            peer_set,
            coding,
            ready_sent: init_ready_sent,
            can_decode: HashSet::new(),
            decided: false,
            output:Vec::new(),
            fault_estimate,
            echos: HashMap::new(),
            //echo_set: HashSet::new(),
            echo_set:init_map,
            readys: HashMap::new(),
        }
    }

    /// 处理收到的Message信息.
    pub async fn handle_message(&mut self, source: &str, message: &Vec<u8>) ->Option<Vec<u8>> {
        let msg = coder::deserialize_for_bytes(message);
        match msg {
            //Message::Request(request) => self.handle_input(request).await,
            Message::Value(id,p) => {
                self.handle_value(id.as_str(), p).await;
                self.get_output()
            },
            Message::Echo(id,e) => {
                self.handle_echo(id.as_str(), e).await;
                self.get_output()
            },
            Message::Ready(id,ready) => {
                self.handle_ready(id.as_str(),source, ready).await;
                self.get_output()
            },
            _ => self.get_output()
        }
        
    }

    //根据输入的input完成reed_solomon纠删码的生成，生成N个纠删码片段，
    //对每个纠删码片段，生成一个Proof并将该Proof封装成一个Value广播
    pub async fn handle_input(&mut self, input:Vec<u8>) {
       
        println!("rbc handle input****************");
        //此处模拟input
        let mut value = vec![12, 32, 0, 23, 32, 8, 12, 55];
        let data_shard_num = self.coding.data_shard_count();
        let parity_shard_num = self.coding.parity_shard_count();
        let payload_len = value.len() as u32;
        value.splice(0..0, 0..4);
        BigEndian::write_u32(&mut value[..4], payload_len); 
        let value_len = value.len(); 
        let shard_len = (value_len + data_shard_num - 1) / data_shard_num;
        value.resize(shard_len * (data_shard_num + parity_shard_num), 0);
        // 将Value分片。
        let shards_iter = value.chunks_mut(shard_len);
        let mut shards: Vec<&mut [u8]> = shards_iter.collect();
        self.coding.encode(&mut shards).expect("wrong shard size");
        println!("分片为：{:?}", shards);
        //Merkle树的建立，Proof的生成与Val广播
        let mtree = MerkleTree::from_vec(shards.into_iter().map(|shard| shard.to_vec()).collect());
        //将proof封装为Val并使用libp2p进行广播
        
        println!("索引为{:?}", self.peer_set.marks);

        for (id, index) in self.peer_set.marks.iter() {
            let proof = mtree.proof(*index).expect("分发错误");
            //println!("merkle_tree{:?}",proof);
            let value = Message::Value(self.my_id.clone(),proof);
            let val = SubsetMessageContent::Broadcast(self.my_id.clone(), value);
            let data = SubsetMessage {
                proposer_id: self.my_id.clone(),
                content: val,
            };
            let serialized_msg = coder::serialize_into_bytes(&data);
            self.broadcast_value(&serialized_msg[..]).await;
            println!("成功发送Value广播");
        }
    }

    //处理节点收到的Value广播
    pub async fn handle_value(&mut self, source: &str, proof: Proof<Vec<u8>>) {
        
        println!("已收到来自节点{}的Value广播", source);
        let dt = Local::now();
        let mut rng = rand::thread_rng();
        let echo = Message::Echo(
            source.to_string(),{
            Echo {
                proof,
                timestamp: dt.timestamp(),
                nonce: rng.gen(),
            }
        });
        let val = SubsetMessageContent::Broadcast(source.to_string(), echo.clone());
        let data = SubsetMessage {
            proposer_id: source.to_string(),
            content: val,
        };
        let serialized_msg = coder::serialize_into_bytes(&data);
        self.broadcast_echo(&serialized_msg[..]).await;
        println!("Echo消息广播成功，内容为：{:?}", echo.clone());
    }

    //处理节点收到的Echo广播
    pub async fn handle_echo(&mut self, source: &str, e: Echo)  {
        
        //收到重复的echo则丢弃
        if let Some(echo) = self.echo_set.get(source).unwrap().get(&e.proof){
            
        }
        //对收到的proof进行验证
        if !self.validate_proof(&e.proof, source) {
            println!("此Echo未通过校验，丢弃");
            
        }
        println!("已收到来自节点{}的Echo消息,内容为：{:?}", source, e.proof);
        let hash = *e.proof.root_hash();
        //保存收到的proof，为以后重构merkle tree做准备
        //self.echo_set.insert(source.to_string(), self.echo_set.get_mut(source.to_string()).insert(e.proof));
        self.echo_set.get_mut(source).unwrap().insert(e.proof);
        //如果收到了f+1个echo消息，可以进行解码
        if self.count_echos(source,&hash) == self.fault_estimate + 1 {
            self.can_decode.insert(hash.clone());

            println!(
                "已收到f+1个echo消息 （{}个），可以进行解码",
                self.count_echos(source,&hash)
            );
        }
        let dt = Local::now();
        let mut rng = rand::thread_rng();
        //如果收到了2f+1个echo并且root hash相同，发送Ready信息
        if !self.ready_sent.get(source).unwrap() && self.count_echos(source,&hash) == 2 * self.fault_estimate + 1 {
            self.ready_sent.entry(source.to_string()).and_modify(|value|{
                *value = true;
            });
            println!("ready_sent: {:?}",self.ready_sent.get(source).unwrap());
            let ready = Message::Ready(source.to_string(),{
                message::Ready {
                    hash: hash,
                    timestamp: dt.timestamp(),
                    nonce: rng.gen(),
                }
            });
            let val = SubsetMessageContent::Broadcast(source.to_string(), ready);
            let value = SubsetMessage {
                proposer_id: source.to_string(),
                content: val,
            };
            let serialized_msg = coder::serialize_into_bytes(&value);
            self.broadcast_ready(&serialized_msg[..]).await;
            println!(
                "已收到来自节点{}的2f+1个echo消息({}个)，广播ready消息成功",
                source,
                self.count_echos(source,&hash)
            );
        }
    }

    pub async fn handle_ready(&mut self, source: &str,from:&str, ready: message::Ready) {

        println!("收到来自节点{}的Ready消息", from);
        self.readys.insert(from.to_string(), ready.hash.to_vec());
        // 收到f+1个ready时广播ready
        if (!self.ready_sent.get(source).unwrap() && self.count_readys(&ready.hash) == self.fault_estimate + 1) {
            println!(
                "已经收到f+1({})个ready信息，进行ready的广播",
                self.count_readys(&ready.hash)
            );
            
            let dt = Local::now();
            let mut rng = rand::thread_rng();
            let ready = Message::Ready(source.to_string(),{
                message::Ready {
                    hash: ready.hash,
                    timestamp: dt.timestamp(),
                    nonce: rng.gen(),
                }
            });
            let val = SubsetMessageContent::Broadcast(source.to_string(), ready);
            let value = SubsetMessage {
                proposer_id: source.to_string(),
                content: val,
            };
            let serialized_msg = coder::serialize_into_bytes(&value);
            self.broadcast_ready(&serialized_msg[..]).await;

            self.ready_sent.entry(source.to_string()).and_modify(|value|{
                *value = true;
            });
        }
        //收到2f+1个ready,并且之前受到过f+1个该hash对应的proof时恢复纠删码
        if (self.count_readys(&ready.hash) == 2 * self.fault_estimate + 1 && self.can_decode.contains(&ready.hash)) {
            println!("收到2f+1个ready信息,hash为{:?}", ready.hash);
            self.output(source,&ready.hash).await;
        }

    }

    async fn output(&mut self,source:&str, hash: &Digest) {
        println!("测试output,排序前\n{:?}", self.echo_set.get(source).unwrap());
        let mut final_set: BTreeMap<usize, &Proof<Vec<u8>>> = BTreeMap::new();

        for echo in self.echo_set.get(source).unwrap(){
            if (echo.root_hash() == hash) {
                final_set.insert(echo.index(), echo);
            }
        }


        let mut leaf_values: Vec<Option<Box<[u8]>>> = final_set
            .iter()
            .map(|echo| {
                if echo.1.root_hash() == hash {
                    Some(echo.1.value().clone().into_boxed_slice())
                } else {
                    None
                }
            })
            .collect();

        println!("leafs:{:?},hash:{:?}", leaf_values, hash);
        if let Some(value) = self.decode_from_shards(&mut leaf_values, hash) {
            println!("解码后得到的消息为：{:?}", value);
            self.decided = true;
            self.output = value;
            println!("{} 节点解码成功，获得交易信息！", self.my_id);
            // println!("已收到RBC消息，发送Complete消息");
            // let complete = Message::Complete(true);
            // let val = SubsetMessageContent::Broadcast(source.to_string(), complete);
            // let value = SubsetMessage {
            //     proposer_id: source.to_string(),
            //     content: val,
            // };
            // let data = coder::serialize_into_bytes(&value);
            // self.broadcast_ready(&data).await;
        } else {
            println!("节点解析交易失败！")
        }
    }

    pub fn get_output(&self) -> Option<Vec<u8>>{
        if(self.decided == true){
            Some(self.output.clone())
        }else {
            None
        }
    }

    //将得到的f+1个分片重组为原始的value
    fn decode_from_shards(
        &self,
        leaf_values: &mut [Option<Box<[u8]>>],
        root_hash: &Digest,
    ) -> Option<Vec<u8>> {
        self.coding
            .reconstruct_shards(leaf_values)
            .expect("分片重组失败！");
        let shards: Vec<Vec<u8>> = leaf_values
            .iter()
            .filter_map(|l| l.as_ref().map(|v| v.to_vec()))
            .collect();
        println!("收集到的分片为:{:?}", shards.clone());
        let mtree = MerkleTree::from_vec(shards.into_iter().map(|shard| shard.to_vec()).collect());

        let count = self.coding.data_shard_count();
        let mut bytes = mtree.into_values().into_iter().take(count).flatten();
        let payload_len = match (bytes.next(), bytes.next(), bytes.next(), bytes.next()) {
            (Some(b0), Some(b1), Some(b2), Some(b3)) => {
                BigEndian::read_u32(&[b0, b1, b2, b3]) as usize
            }
            _ => return None, 
        };
        println!("payload is {}", payload_len);
        let payload: Vec<u8> = bytes.take(payload_len).collect();
        Some(payload)
    }

    fn count_echos(&self, source:&str,hash: &Digest) -> usize {
        self.echo_set.get(source)
        .unwrap()
        .iter()
        .filter(|echo| echo.root_hash() == hash)
        .count()

    }

    fn count_readys(&self, hash: &Digest) -> usize {
        self.readys
            .values()
            .filter(|h| h.as_slice() == hash)
            .count()
    }

    pub async fn broadcast_value(&self, msg: &[u8]) {
        self.msg_tx.send(msg.to_vec()).await.unwrap();
    }

    pub async fn broadcast_echo(&self, msg: &[u8]) {
        self.msg_tx.send(msg.to_vec()).await.unwrap();
    }

    pub async fn broadcast_ready(&self, msg: &[u8]) {
        self.msg_tx.send(msg.to_vec()).await.unwrap();
    }

    pub async fn broadcast_can_decode(&self, msg: &[u8]) {
        self.msg_tx.send(msg.to_vec()).await.unwrap();
    }

    //节点对proof进行自验证保证完整性
    fn validate_proof(&mut self, proof: &Proof<Vec<u8>>, id: &str) -> bool {
        //self.peer_set.index(id) == Some(proof.index()) && proof.validate(self.peer_set.num());
        proof.validate(self.peer_set.num())
    }
}

#[derive(Debug)]
enum Coding {
    ReedSolomon(Box<ReedSolomon<Field8>>),
    Trivial(usize),
}

impl Coding {
    fn new(data_shard_num: usize, parity_shard_num: usize) -> RseResult<Self> {
        Ok(if parity_shard_num > 0 {
            let rs = ReedSolomon::new(data_shard_num, parity_shard_num)?;
            Coding::ReedSolomon(Box::new(rs))
        } else {
            Coding::Trivial(data_shard_num)
        })
    }

    fn data_shard_count(&self) -> usize {
        match *self {
            Coding::ReedSolomon(ref rs) => rs.data_shard_count(),
            Coding::Trivial(dsc) => dsc,
        }
    }

    fn parity_shard_count(&self) -> usize {
        match *self {
            Coding::ReedSolomon(ref rs) => rs.parity_shard_count(),
            Coding::Trivial(_) => 0,
        }
    }

    fn encode(&self, slices: &mut [&mut [u8]]) -> RseResult<()> {
        match *self {
            Coding::ReedSolomon(ref rs) => rs.encode(slices),
            Coding::Trivial(_) => Ok(()),
        }
    }

    fn reconstruct_shards(&self, shards: &mut [Option<Box<[u8]>>]) -> RseResult<()> {
        match *self {
            Coding::ReedSolomon(ref rs) => rs.reconstruct(shards),
            Coding::Trivial(_) => {
                if shards.iter().all(Option::is_some) {
                    Ok(())
                } else {
                    Err(rse::Error::TooFewShardsPresent)
                }
            }
        }
    }
}

#[derive(Debug)]
enum EchoContent {
    Hash(Digest),
    Full(Proof<Vec<u8>>),
}

impl EchoContent {
    pub fn hash(&self) -> &Digest {
        match &self {
            EchoContent::Hash(h) => h,
            EchoContent::Full(p) => p.root_hash(),
        }
    }

    pub fn proof(&self) -> Option<&Proof<Vec<u8>>> {
        match &self {
            EchoContent::Hash(_) => None,
            EchoContent::Full(p) => Some(p),
        }
    }
}

#[cfg(test)]
mod reed_solomon_erasure_test {
    use super::*;
    #[test]
    fn reed_solomon() {
        let parity_shard_num = 2;
        let data_shard_num = 2;
        let coding = Coding::new(data_shard_num, parity_shard_num).expect("msg");
        let mut value = vec![12, 32, 123, 43, 12, 43, 12, 43, 12, 12, 12, 12];
        let data_shard_num = 2;
        let parity_shard_num = 2;
        let payload_len = value.len() as u32;
        value.splice(0..0, 0..4);
        BigEndian::write_u32(&mut value[..4], payload_len);
        let value_len = value.len();

        let shard_len = (value_len + data_shard_num - 1) / data_shard_num;
        value.resize(shard_len * (data_shard_num + parity_shard_num), 0);

        let shards_iter = value.chunks_mut(shard_len);
        let mut shards: Vec<&mut [u8]> = shards_iter.collect();

        coding.encode(&mut shards).expect("wrong shard size");

        println!(
            "Value: {} bytes, {} per shard. Shards: {:?}",
            value_len, shard_len, shards,
        );

        let mtree = MerkleTree::from_vec(shards.into_iter().map(|shard| shard.to_vec()).collect());

        let proof = mtree.proof(0).expect("msg");
        println!("{:?}", proof);
    }
}
