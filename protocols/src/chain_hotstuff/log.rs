use std::collections::HashMap;

use threshold_crypto::SignatureShare;

use super::message::{QC, Generic};

pub struct Log {
    pub message_signatures: HashMap<String, HashMap<u64, SignatureShare>>,
    pub generics: HashMap<u64, Vec<Generic>>,
}

impl Log {
    pub fn new() -> Self {
        Self {
            message_signatures: HashMap::new(),
            generics: HashMap::new(),
        }
    }

    pub fn record_generic(&mut self, view_num: u64, generic: &Generic) {
        if let Some(generics) = self.generics.get_mut(&view_num) {
            generics.push(generic.clone());
        } else {
            let generic_vec = vec![generic.clone()];
            self.generics.insert(view_num, generic_vec);
        }
    }

    pub fn get_generics_count_by_view(&self, view_num: u64) -> usize {
        if let Some(generics) = self.generics.get(&view_num) {
            generics.len()
        } else {
            0
        }
    }

    // get high_qc by view number in local logs
    pub fn get_high_qc_by_view(&self, view_num: u64, generic_qc: Option<QC>) -> Option<QC> {
        let generic_vec = self.generics.get(&view_num).unwrap();
        let mut high_qc: Option<QC> = generic_qc;

        for generic in generic_vec {
            if generic.justify.is_none() {
                continue;
            }

            if high_qc.is_none() {
                high_qc = generic.justify.clone();
                continue;
            }

            if high_qc.as_ref().unwrap().view_num < generic.justify.as_ref().unwrap().view_num {
                high_qc = generic.justify.clone();
            }
        }

        high_qc
    }

    // record vote partial signature
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
