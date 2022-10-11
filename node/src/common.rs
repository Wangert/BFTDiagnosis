use std::{cell::RefCell, collections::HashMap, rc::Rc};

use chrono::Local;
use libp2p::PeerId;

use utils::{
    coder,
    crypto::threshold_blsttc::{self, TBLSKey},
};

use crate::message::{InteractiveMessage, Message};

use super::message::{Command, CommandMessage, DistributeTBLSKey, Request};

#[derive(Debug, Clone)]
pub struct Block {
    pub cmd: String,
    pub parent: RefCell<Rc<Block>>,
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

pub fn generate_a_consensus_request_command() -> CommandMessage {
    let timestamp = Local::now().timestamp_nanos() as u64;
    let cmd = format!("{}{}", "wangjitao", timestamp);
    let request = Request { cmd, timestamp};

    let message = CommandMessage {
        command: Command::MakeAConsensusRequest(request),
    };

    message
}

pub fn generate_consensus_requests_command(size: usize) -> Message {
    let mut requests = vec![];
    for _ in 0..size {
        let timestamp = Local::now().timestamp_nanos() as u64;
        let cmd = format!("{}{}", "wangjitao", timestamp);
        let request = Request { cmd, timestamp };
        requests.push(request);
    }

    let message = Message {
        interactive_message: InteractiveMessage::MakeConsensusRequests(requests),
        source: vec![],
    };

    message
}

pub fn get_request_hash(request: &Request) -> String {
    let serialized_request = coder::serialize_into_bytes(request);
    coder::get_hash_str(&serialized_request)
}

#[cfg(test)]
mod node_common_tests {

    use chrono::prelude::*;

    #[test]
    fn time_diff_works() {
        let start = Local::now().timestamp_millis() as u64;
        //sleep_ms(2);
        let end = Local::now().timestamp_millis() as u64;

        println!("{:?}", end - start);

        // let start_datetime = Local.timestamp_nanos(start);
        // let end_datetime = Local.timestamp_nanos(end);
        // let diff = end_datetime - start_datetime;
        // println!("{:?}ms", diff.num_milliseconds());
    }
}
