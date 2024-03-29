use std::collections::HashMap;

use blsttc::serde_impl::SerdeSecret;

use blsttc::{
    PublicKeySet, PublicKeyShare, SecretKeySet, SecretKeyShare, Signature, SignatureShare,PublicKey}
;
use serde::{Deserialize, Serialize};

pub struct ThresholdSigKeys {
    pub keypair_shares: Vec<(u64, SerdeSecret<SecretKeyShare>, PublicKeyShare)>,
    pub common_pk: PublicKey,
    pub pk_set: PublicKeySet,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TBLSKey {
    pub secret_key: SerdeSecret<SecretKeyShare>,
    pub public_key: PublicKeyShare,
    pub common_public_key: PublicKey,
    pub pk_set: PublicKeySet,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TBLSSignature {
    pub number: u64,
    pub signature: SignatureShare,
}

impl TBLSKey {
    pub fn sign(&self, data: &[u8]) -> SignatureShare {
        self.secret_key.sign(data)
    }

    pub fn partial_verify(&self, sig: &SignatureShare, data: &[u8]) -> bool {
        self.public_key.verify(sig, data)
    }

    pub fn threshold_verify(&self, sig: &Signature, data: &[u8]) -> bool {
        self.common_public_key.verify(sig, data)
    }

    pub fn combine_partial_signatures(&self, sigs: &HashMap<u64, SignatureShare>) -> Signature {
        self.pk_set
            .combine_signatures(sigs)
            .expect("Combine partial signatures error!")
    }
}

pub fn generate_keypair_set(t: usize, n: usize) -> ThresholdSigKeys {
    // let mut rng = rand::thread_rng();
    let sk_set = SecretKeySet::random(t, &mut rand::thread_rng());
    let pk_set = sk_set.public_keys();
    let sk_and_pk_shares: Vec<_> = (0..n)
        .map(|i| {
            (
                i as u64,
                SerdeSecret(sk_set.secret_key_share(i)),
                pk_set.public_key_share(i),
            )
        })
        .collect();

    let common_pk = pk_set.public_key();

    ThresholdSigKeys {
        keypair_shares: sk_and_pk_shares,
        common_pk,
        pk_set,
    }
}

pub fn sign(sk_share: &SecretKeyShare, msg: &str) -> SignatureShare {
    sk_share.sign(msg)
}

pub fn combine_partial_signatures(
    pk_set: &PublicKeySet,
    sigs: &HashMap<u64, SignatureShare>,
) -> Signature {
    pk_set
        .combine_signatures(sigs)
        .expect("Combine partial signatures error!")
}

#[cfg(test)]
mod threshold_sig_tests {
    use std::{collections::HashMap, time::SystemTime};

    use super::*;
    #[test]
    fn generate_works() {
        let t_sig_keys = generate_keypair_set(3, 9);
        for (_, sks, pks) in t_sig_keys.keypair_shares {
            println!("【sk:{:?}, pk:{:?}】", &sks.reveal(), &pks);
        }
    }

    #[test]
    fn combine_sigs_works() {
        let t_sig_keys = generate_keypair_set(3, 9);
        let msg = "wangjiato";

        let nodes = t_sig_keys.keypair_shares;

        // let node_1_keys = (*t_sig_keys.keypair_shares).get(0).unwrap();
        // let node_2_keys = (*t_sig_keys.keypair_shares).get(1).unwrap();
        // let node_3_keys = (*t_sig_keys.keypair_shares).get(3).unwrap();
        // let node_4_keys = (*t_sig_keys.keypair_shares).get(4).unwrap();

        // let mut sigs = BTreeMap::new();

        let n = nodes.clone();

        let sigs: HashMap<_, _> = n
            .into_iter()
            .map(|(i, sk, _)| (i, sign(&sk, msg)))
            .collect();

        // let mut sigs: BTreeMap<_, _> = n
        //     .into_iter()
        //     .map(|(i, sk, _)| (i, sign(&sk, msg)))
        //     .collect();
        // sigs.insert(0, sign(&node_1_keys.0, msg));
        // sigs.insert(1, sign(&node_2_keys.0, msg));
        // sigs.insert(3, sign(&node_3_keys.0, msg));
        // sigs.insert(4, sign(&node_4_keys.0, msg));

        println!("=============Partial Verification===========");

        for (i, _, pk) in nodes.into_iter() {
            println!("{}: {}", i, pk.verify(&sigs.get(&i).unwrap(), msg));
        }

        // let sys_time3 = SystemTime::now();
        // // println!("1: {}", node_1_keys.1.verify(&sigs.get(&0).unwrap(), msg));
        // let sys_time4 = SystemTime::now();
        // let difference1 = sys_time2.duration_since(sys_time1);
        // println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        // println!("Verify time spent: {:?}", difference1);
        // println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        // println!("2: {}", node_2_keys.1.verify(&sigs.get(&1).unwrap(), msg));
        // println!("3: {}", node_3_keys.1.verify(&sigs.get(&3).unwrap(), msg));
        // println!("4: {}", node_4_keys.1.verify(&sigs.get(&4).unwrap(), msg));

        // sigs.remove(&2);
        // sigs.remove(&5);
        // sigs.remove(&7);
        // sigs.remove(&0);
        // sigs.remove(&3);
        // sigs.remove(&6);
        for (i, sig) in &sigs {
            println!("{}|sig:[{:?}]|", i, &sig);
        }

        let sys_time1 = SystemTime::now();
        let com_sig = combine_partial_signatures(&t_sig_keys.pk_set, &sigs);
        let sys_time2 = SystemTime::now();

        let difference = sys_time2.duration_since(sys_time1);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        println!("Combined time spent: {:?}", difference);
        println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
        println!("Combined Signature: [{:?}]", &com_sig);

        println!("=============Threshold Signature Verification===========");
        println!("result: {}", t_sig_keys.common_pk.verify(&com_sig, msg));
    }
}
