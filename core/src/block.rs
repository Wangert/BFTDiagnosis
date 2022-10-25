use chrono::prelude::*;
use serde::{Serialize, Deserialize};
use utils::coder;

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockHeader {
    pub timestamp: i64,
    pub transaction_hash : [u8;32],
    pub pre_hash: [u8;32],

}

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub hash: [u8;32],
    pub transaction: String,
}

impl Block {
    pub fn new(pre_hash: [u8;32], transaction: String) -> Block {
        let transactions = coder::block_serialize(&transaction);
        let timestamp = Utc::now().timestamp();
        let mut transaction_hash: [u8; 32] = [0; 32];
        coder::get_hash_u8_vec(&transactions[..], &mut transaction_hash);
        let header = BlockHeader {
            timestamp,
            transaction_hash,
            pre_hash,
        };

        Block {
            header,
            hash: [0; 32],
            transaction,
        }
    }

    
}
