use std::collections::HashMap;

use libp2p::PeerId;
use utils::{coder, crypto::threshold_blsttc::{self, TBLSKey}};

// use super::message::{Block, Request};
use crate::chain_hotstuff::message::{Block};
use components::message::{Request, DistributeTBLSKey};
pub const GENERIC: u8 = 1;
// pub const PREPARE: u8 = 1;
// pub const PRE_COMMIT: u8 = 2;
// pub const COMMIT: u8 = 3;
// pub const DECIDE: u8 = 4;

pub fn get_block_hash(block: &Block) -> String {
    let serialized_block = coder::serialize_into_bytes(block);
    coder::get_hash_str(&serialized_block)
}

pub fn get_request_hash(request: &Request) -> String {
    let serialized_request = coder::serialize_into_bytes(request);
    coder::get_hash_str(&serialized_request)
}

pub fn get_generic_hash(msg_type: u8, view_num: u64, block: &Block) -> String {
    if msg_type != GENERIC {
        eprintln!("Generic type is error!");
        return "".to_string();
    }

    let mut hash_vec = msg_type.to_be_bytes().to_vec();
    let mut view_num_vec = view_num.to_be_bytes().to_vec();
    let mut block_vec = coder::serialize_into_bytes(block);

    hash_vec.append(&mut view_num_vec);
    hash_vec.append(&mut block_vec);

    coder::get_hash_str(&hash_vec)
}

pub fn generate_bls_keys(
    consensus_nodes: &HashMap<String, PeerId>,
    fault_count: u64,
) -> Vec<DistributeTBLSKey> {
    let key_set = threshold_blsttc::generate_keypair_set(
        fault_count as usize,
        consensus_nodes.len() as usize,
    );
    key_set
        .keypair_shares
        .iter()
        .zip(consensus_nodes.iter())
        .map(|((i, sk, pk), (_, peer_id))| {
            let tbls_key = TBLSKey {
                secret_key: sk.clone(),
                public_key: *pk,
                common_public_key: key_set.common_pk,
                pk_set: key_set.pk_set.clone(),
            };
            DistributeTBLSKey {
                number: *i,
                peer_id: peer_id.clone(),
                tbls_key,
            }
        })
        .collect::<Vec<DistributeTBLSKey>>()
}

// pub fn create_requests(size: usize) -> Vec<Message> {
//     let mut msg_vec = vec![];
//     for _ in 0..size {
//         let cmd = format!("{}{}", "wangjitao", Local::now().timestamp_subsec_nanos());
//         let request = Request { cmd };

//         let msg = Message {
//             msg_type: MessageType::Request(request),
//         };

//         msg_vec.push(msg);
//     }

//     msg_vec
// }

