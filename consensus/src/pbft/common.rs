use utils::coder::{self, get_hash_str};

use super::message::MessageType;

pub const REQUEST_PREFIX: u8 = 0;
pub const PRE_PREPARE_PREFIX: u8 = 1;
pub const PREPARE_PREFIX: u8 = 2;
pub const COMMIT_PREFIX: u8 = 3;
pub const REPLY_PREFIX: u8 = 4;

pub fn get_message_key(msg_type: MessageType) -> String {
    match msg_type {
        MessageType::Request(request) => {
            let serialized_request = coder::serialize_into_bytes(&request);
            get_hash_str(&serialized_request)
        }
        MessageType::PrePrepare(preprepare) => {
            let mut key = vec![PRE_PREPARE_PREFIX];
            let mut view_vec = preprepare.view.to_be_bytes().to_vec();
            let mut number_vec = preprepare.number.to_be_bytes().to_vec();
            let mut m_hash_vec = preprepare.m_hash.into_bytes();
            key.append(&mut view_vec);
            key.append(&mut number_vec);
            key.append(&mut m_hash_vec);

            get_hash_str(&key)
        }
        MessageType::Prepare(prepare) => {
            let mut key = vec![PREPARE_PREFIX];
            let mut view_vec = prepare.view.to_be_bytes().to_vec();
            let mut number_vec = prepare.number.to_be_bytes().to_vec();
            let mut m_hash_vec = prepare.m_hash.into_bytes();
            key.append(&mut view_vec);
            key.append(&mut number_vec);
            key.append(&mut m_hash_vec);

            get_hash_str(&key)
        }
        MessageType::Commit(commit) => {
            let mut key = vec![COMMIT_PREFIX];
            let mut view_vec = commit.view.to_be_bytes().to_vec();
            let mut number_vec = commit.number.to_be_bytes().to_vec();
            let mut m_hash_vec = commit.m_hash.into_bytes();
            key.append(&mut view_vec);
            key.append(&mut number_vec);
            key.append(&mut m_hash_vec);

            get_hash_str(&key)
        }
        MessageType::Reply(reply) => "".to_string(),
    }
}
