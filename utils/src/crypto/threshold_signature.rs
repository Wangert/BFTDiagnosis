use std::collections::HashMap;

use threshold_crypto::{
    PublicKey, PublicKeySet, PublicKeyShare, SecretKeySet, SecretKeyShare, Signature,
    SignatureShare,
};

pub struct ThresholdSigKeys {
    pub keypair_shares: Box<Vec<(u64, SecretKeyShare, PublicKeyShare)>>,
    pub common_pk: PublicKey,
    pub pk_set: PublicKeySet,
}

pub fn generate_keypair_set(t: usize, n: usize) -> ThresholdSigKeys {
    let mut rng = rand::thread_rng();
    let sk_set = SecretKeySet::random(t, &mut rng);
    let pk_set = sk_set.public_keys();
    let sk_and_pk_shares: Vec<_> = (0..n)
        .map(|i| (i as u64, sk_set.secret_key_share(i), pk_set.public_key_share(i)))
        .collect();

    // for (sks, pks) in &sk_and_pk_shares {
    //     println!("【sk:{:?}, pk:{:?}】", &sks.reveal(), &pks);
    // }

    let common_pk = pk_set.public_key();

    ThresholdSigKeys {
        keypair_shares: Box::new(sk_and_pk_shares),
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
    use std::collections::HashMap;

    use super::*;
    #[test]
    fn generate_works() {
        let t_sig_keys = generate_keypair_set(3, 9);
        for (_, sks, pks) in *t_sig_keys.keypair_shares {
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

        let mut sigs: HashMap<_, _> = n.into_iter().map(|(i, sk, _)| (i, sign(&sk, msg))).collect();

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

        // println!("1: {}", node_1_keys.1.verify(&sigs.get(&0).unwrap(), msg));
        // println!("2: {}", node_2_keys.1.verify(&sigs.get(&1).unwrap(), msg));
        // println!("3: {}", node_3_keys.1.verify(&sigs.get(&3).unwrap(), msg));
        // println!("4: {}", node_4_keys.1.verify(&sigs.get(&4).unwrap(), msg));

        sigs.remove(&2);
        sigs.remove(&5);
        sigs.remove(&7);
        sigs.remove(&0);
        sigs.remove(&3);
        // sigs.remove(&6);
        for (i, sig) in &sigs {
            println!("{}|sig:[{:?}]|", i, &sig);
        }
        

        let com_sig = combine_partial_signatures(&t_sig_keys.pk_set, &sigs);
        println!("Combined Signature: [{:?}]", &com_sig);

        println!("=============Threshold Signature Verification===========");
        println!("result: {}", t_sig_keys.common_pk.verify(&com_sig, msg));
    }
}
