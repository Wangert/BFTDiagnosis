use utils::coder;

use super::message::{Block, Request};

pub const REQUEST: u8 = 0;
pub const PREPARE: u8 = 1;
pub const PRE_COMMIT: u8 = 2;
pub const COMMIT: u8 = 3;
pub const DECIDE: u8 = 4;

pub fn get_block_hash(block: &Block) -> String {
    let serialized_block = coder::serialize_into_bytes(block);
    coder::get_hash_str(&serialized_block)
}

pub fn get_request_hash(request: &Request) -> String {
    let serialized_request = coder::serialize_into_bytes(request);
    coder::get_hash_str(&serialized_request)
}

pub fn get_message_hash(msg_type: u8, view_num: u64, block: &Block) -> String {
    if msg_type > 4 {
        eprintln!("Message type is error!");
        return "".to_string();
    }

    let mut hash_vec = msg_type.to_be_bytes().to_vec();
    let mut view_num_vec = view_num.to_be_bytes().to_vec();
    let mut block_vec = coder::serialize_into_bytes(block);

    hash_vec.append(&mut view_num_vec);
    hash_vec.append(&mut block_vec);

    coder::get_hash_str(&hash_vec)
}
