use utils::coder::{self, get_hash_str};

use super::message::MessageType;

pub const REQUEST_PREFIX: u8 = 0;
pub const PRE_PREPARE_PREFIX: u8 = 1;
pub const PREPARE_PREFIX: u8 = 2;
pub const COMMIT_PREFIX: u8 = 3;
pub const REPLY_PREFIX: u8 = 4;

pub const STABLE_CHECKPOINT_DELTA: u64 = 50;

// get preprepare key by request hash, view, sequence number in local logs
pub fn get_preprepare_key_by_request_hash(
    request_hash: &[u8],
    view: u64,
    sequence_number: u64,
) -> String {
    let mut key = vec![PRE_PREPARE_PREFIX];
    let mut view_vec = view.to_be_bytes().to_vec();
    let mut number_vec = sequence_number.to_be_bytes().to_vec();
    let mut m_hash_vec = request_hash.to_vec();
    key.append(&mut view_vec);
    key.append(&mut number_vec);
    key.append(&mut m_hash_vec);

    get_hash_str(&key)
}

// get prepare key by request hash, view, sequence number in local logs
pub fn get_prepare_key_by_request_hash(
    request_hash: &[u8],
    view: u64,
    sequence_number: u64,
) -> String {
    let mut key = vec![PREPARE_PREFIX];
    let mut view_vec = view.to_be_bytes().to_vec();
    let mut number_vec = sequence_number.to_be_bytes().to_vec();
    let mut m_hash_vec = request_hash.to_vec();
    key.append(&mut view_vec);
    key.append(&mut number_vec);
    key.append(&mut m_hash_vec);

    get_hash_str(&key)
}

// get commit key by request hash, view, sequence number in local logs
pub fn get_commit_key_by_request_hash(
    request_hash: &[u8],
    view: u64,
    sequence_number: u64,
) -> String {
    let mut key = vec![COMMIT_PREFIX];
    let mut view_vec = view.to_be_bytes().to_vec();
    let mut number_vec = sequence_number.to_be_bytes().to_vec();
    let mut m_hash_vec = request_hash.to_vec();
    key.append(&mut view_vec);
    key.append(&mut number_vec);
    key.append(&mut m_hash_vec);

    get_hash_str(&key)
}

// get message key
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
        MessageType::Reply(reply) => {
            let mut key = vec![REPLY_PREFIX];
            let mut client_id_vec = reply.client_id;
            let mut timestamp_vec = reply.timestamp.into_bytes();
            let mut view_vec = reply.view.to_be_bytes().to_vec();
            let mut number_vec = reply.number.to_be_bytes().to_vec();
            let mut result_vec = reply.result;
            key.append(&mut client_id_vec);
            key.append(&mut timestamp_vec);
            key.append(&mut view_vec);
            key.append(&mut number_vec);
            key.append(&mut result_vec);

            get_hash_str(&key)
        }
        _ => String::from(""),
    }
}
