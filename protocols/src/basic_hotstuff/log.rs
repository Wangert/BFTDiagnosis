use std::collections::HashMap;

use blsttc::SignatureShare;

use super::message::{NewView, QC};

pub struct Log {
    pub message_signatures: HashMap<String, HashMap<u64, SignatureShare>>,
    pub newviews: HashMap<u64, Vec<NewView>>,
}

impl Log {
    pub fn new() -> Self {
        Self {
            message_signatures: HashMap::new(),
            newviews: HashMap::new(),
        }
    }

    pub fn record_newview(&mut self, view_num: u64, newview: &NewView) {
        if let Some(newviews) = self.newviews.get_mut(&view_num) {
            newviews.push(newview.clone());
        } else {
            let newview_vec = vec![newview.clone()];
            self.newviews.insert(view_num, newview_vec);
        }
    }

    pub fn get_newviews_count_by_view(&self, view_num: u64) -> usize {
        if let Some(newviews) = self.newviews.get(&view_num) {
            newviews.len()
        } else {
            0
        }
    }

    pub fn get_high_qc_by_view(&self, view_num: u64, prepare_qc: Option<QC>) -> Option<QC> {
        let newview_vec = self.newviews.get(&view_num).unwrap();
        let mut high_qc: Option<QC> = prepare_qc;

        for newview in newview_vec {
            if newview.justify.is_none() {
                continue;
            }

            if high_qc.is_none() {
                high_qc = newview.justify.clone();
                continue;
            }

            if high_qc.as_ref().unwrap().view_num < newview.justify.as_ref().unwrap().view_num {
                high_qc = newview.justify.clone();
            }
        }

        high_qc
    }

    pub fn record_messgae_partial_signature(
        &mut self,
        msg_hash: &str,
        keypair_num: u64,
        partial_sig: &SignatureShare,
    ) {
        if let Some(signatures) = self.message_signatures.get_mut(msg_hash) {
            signatures.insert(keypair_num, partial_sig.clone());
        } else {
            let mut hmap = HashMap::new();
            hmap.insert(keypair_num, partial_sig.clone());
            self.message_signatures.insert(msg_hash.to_string(), hmap);
        }
    }

    pub fn get_partial_signatures_by_message_hash(
        &self,
        msg_hash: &str,
    ) -> &HashMap<u64, SignatureShare> {
        self.message_signatures
            .get(msg_hash)
            .expect("Message partial signatures are not found!")
    }

    pub fn get_partial_signatures_count_by_message_hash(&self, msg_hash: &str) -> usize {
        if let Some(signatures) = self.message_signatures.get(msg_hash) {
            signatures.len()
        } else {
            0
        }
    }
}
