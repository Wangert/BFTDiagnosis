use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
pub struct EdDSAKeyPair(pub Keypair);

impl EdDSAKeyPair {
    pub fn new() -> EdDSAKeyPair {
        let mut csprng = OsRng {};
        let keypair: Keypair = Keypair::generate(&mut csprng);

        EdDSAKeyPair(keypair)
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signature: Signature = self.0.sign(message);
        signature.to_bytes().to_vec()
    }

    pub fn verify(&self, signature: &[u8], message: &[u8]) -> bool {
        let signature_result = Signature::from_bytes(signature);
        let signature = if let Ok(signature) = signature_result {
            signature
        } else {
            eprintln!("Sinature bytes error!");
            return false;
        };

        self.0.verify(message, &signature).is_ok()
    }

    pub fn get_public_key(&self) -> EdDSAPublicKey {
        EdDSAPublicKey(self.0.public.to_bytes().to_vec())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EdDSAPublicKey(pub Vec<u8>);

impl EdDSAPublicKey {
    pub fn new(key: &[u8]) -> EdDSAPublicKey {
        if key.len() == 32 {
            EdDSAPublicKey(key.to_vec())
        } else {
            eprintln!("New eddsa public key error!");
            EdDSAPublicKey(vec![])
        }
    }

    pub fn verify(&self, signature: &[u8], message: &[u8]) -> bool {
        let pk_result = PublicKey::from_bytes(&self.0);
        let pk = if let Ok(pk) = pk_result {
            pk
        } else {
            eprintln!("public key bytes error!");
            return false;
        };

        let signature_result = Signature::from_bytes(signature);
        let signature = if let Ok(signature) = signature_result {
            signature
        } else {
            eprintln!("Sinature bytes error!");
            return false;
        };

        pk.verify(message, &signature).is_ok()
    }

    pub fn from_bytes(public_key_bytes: &[u8]) -> Self {
        EdDSAPublicKey(public_key_bytes.to_vec())
    }
}

#[cfg(test)]
mod eddsa_test {
    use super::EdDSAKeyPair;

    #[test]
    fn signature_verify_works() {
        let eddsa_keypair_1 = EdDSAKeyPair::new();
        let _eddsa_keypair_2 = EdDSAKeyPair::new();

        let eddsa_public_key_1 = eddsa_keypair_1.get_public_key();
        // let public_key_1: PublicKey = eddsa_keypair_1.0.public;
        // let public_key_2: PublicKey = eddsa_keypair_2.0.public;
        // let eddsa_public_key_1 = EdDSAPublicKey::new(public_key_1);
        // let eddsa_public_key_2 = EdDSAPublicKey::new(public_key_2);

        // let eddsa_public_key_1_bytes = eddsa_public_key_1.to_bytes();
        // let eddsa_public_key_11 = EdDSAPublicKey::from_bytes(&eddsa_public_key_1_bytes);
        let message = "wangjitao".as_bytes();

        let signature_1 = eddsa_keypair_1.sign(message);

        assert!(eddsa_public_key_1.verify(&signature_1, message));
        //assert!(eddsa_public_key_11.verify(&signature_1, message));
        //assert!(eddsa_public_key_2.verify(&signature_1, message));
    }
}
