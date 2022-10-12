use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Request {
    pub cmd: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrePrepare {
    pub cmd: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prepare {
    pub cmd: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Commit {
    pub cmd: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConsensusMessage {
    pub msg: MessageType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Reply {
    pub cmd: String,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    Request(Request),
    PrePrepare(PrePrepare),
    Prepare(Prepare),
    Commit(Commit),
    Reply(Reply),
}

