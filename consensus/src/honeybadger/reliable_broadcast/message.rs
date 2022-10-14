use std::fmt::{self, Debug};

use hex_fmt::HexFmt;
use rand::distributions::{Distribution, Standard};
use rand::{self,seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};
use super::merkle::{Digest,MerkleTree,Proof};


//列出了RBC中出现的三种消息类型
#[derive(Debug,Serialize, Deserialize, Clone)]
pub enum Message {
    // Request(Request),
    /// A share of the value, sent from the sender to another validator.
    Value(String,Proof<Vec<u8>>),
    /// A copy of the value received from the sender, multicast by a validator.
    Echo(String,Echo),
    /// Indicates that the sender knows that every node will eventually be able to decode.
    Ready(String,Ready),
    /// 表示该节点有足够的证据来解码给定Merkle根的信息。
    CanDecode(String,Digest),
    Complete(bool),
    Reply(Reply),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Echo{
    pub proof: Proof<Vec<u8>>,
    pub timestamp: i64,
    pub nonce: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone,PartialEq)]
pub struct Ready{
    pub hash:Digest,
    pub timestamp: i64,
    pub nonce: u8,
}


// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct Request {
//     pub data: String,
//     pub client_id: Vec<u8>,
// }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Reply {
    pub client_id: Vec<u8>,
    pub result: Vec<u8>,
}


pub struct HexProof<'a, T>(pub &'a Proof<T>);

impl<'a, T: AsRef<[u8]>> fmt::Debug for HexProof<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Proof {{ #{}, root_hash: {:0.10}, value: {:0.10}, .. }}",
            &self.0.index(),
            HexFmt(self.0.root_hash()),
            HexFmt(self.0.value())
        )
    }
}
