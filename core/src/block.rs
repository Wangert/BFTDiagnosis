use chrono::prelude::*;
use serde::{Serialize, Deserialize};
use utils::coder;

#[derive(Debug, Serialize, Deserialize)]
struct BlockHeader {
    pub timestamp: i64,
    pub pre_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Block {
    pub header: BlockHeader,
    pub hash: String,
    pub data: String,
}

impl Block {
    pub fn new(pre_hash: String, data: String) -> Block {
        let timestamp = Utc::now().timestamp();
        let header = BlockHeader {
            timestamp,
            pre_hash,
        };

        Block {
            header,
            hash: "".to_string(),
            data,
        }
    }

    pub fn generate_hash(&mut self) {
        let header_serialized = coder::serialize_to_bytes(&self.header);
        self.hash = coder::get_sha256(&header_serialized[..]);
    }
}
