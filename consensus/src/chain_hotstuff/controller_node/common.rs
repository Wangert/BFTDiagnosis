use std::{cell::RefCell, collections::HashMap, rc::Rc};

use chrono::Local;
use libp2p::PeerId;

use utils::crypto::threshold_signature::{self, TBLSKey};

use super::message::{DistributeTBLSKey, Message, MessageType, Request};

#[derive(Debug, Clone)]
pub struct Block {
    pub cmd: String,
    pub parent: RefCell<Rc<Block>>,
}

pub fn generate_bls_keys(
    consensus_nodes: &HashMap<String, PeerId>,
    fault_count: u64,
) -> Vec<DistributeTBLSKey> {
    let key_set = threshold_signature::generate_keypair_set(
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

pub fn create_requests(size: usize) -> Vec<Message> {
    let mut msg_vec = vec![];
    for _ in 0..size {
        let cmd = format!("{}{}", "wangjitao", Local::now().timestamp_subsec_nanos());
        let request = Request { cmd };

        let msg = Message {
            msg_type: MessageType::Request(request),
        };

        msg_vec.push(msg);
    }

    msg_vec
}
