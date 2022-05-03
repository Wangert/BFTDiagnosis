use rand::rngs::OsRng;
use ed25519_dalek::{Keypair, Signature, Signer, PublicKey, Verifier};
pub struct EdDSAKeyPair(pub Keypair);

impl EdDSAKeyPair {
    fn new() -> EdDSAKeyPair {
        let mut csprng = OsRng{};
        let keypair: Keypair = Keypair::generate(&mut csprng);

        EdDSAKeyPair(keypair)
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        let signature: Signature = self.0.sign(message);
        signature
    }

    pub fn verify(&self, signature: &Signature, message: &[u8]) -> bool {
        self.0.verify(message, signature).is_ok()
    }
}

pub struct EdDSAPublicKey(pub PublicKey);

impl EdDSAPublicKey {
    fn new(key: PublicKey) -> EdDSAPublicKey {
        EdDSAPublicKey(key)
    }

    fn verify(&self, signature: &Signature, message: &[u8]) -> bool {
        self.0.verify(message, signature).is_ok()
    }
}


#[cfg(test)]
mod eddsa_test {
    use super::{EdDSAKeyPair, EdDSAPublicKey};
    use ed25519_dalek::PublicKey;

    #[test]
    fn signature_verify_works() {
        let eddsa_keypair_1 = EdDSAKeyPair::new();
        let eddsa_keypair_2 = EdDSAKeyPair::new();

        let public_key_1: PublicKey = eddsa_keypair_1.0.public;
        let public_key_2: PublicKey = eddsa_keypair_2.0.public;
        let eddsa_public_key_1 = EdDSAPublicKey::new(public_key_1);
        let eddsa_public_key_2 = EdDSAPublicKey::new(public_key_2);
 

        let message = "wangjitao".as_bytes();

        let signature_1 = eddsa_keypair_1.sign(message);
        
        assert!(eddsa_public_key_1.verify(&signature_1, message));
        //assert!(eddsa_public_key_2.verify(&signature_1, message));
    }
}