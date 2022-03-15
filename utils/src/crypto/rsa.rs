use rand::rngs::OsRng;
use rsa::{PaddingScheme, PublicKey, RSAPrivateKey, RSAPublicKey};

pub struct Rsa {
    rng: OsRng,
    private_key: RSAPrivateKey,
    public_key: RSAPublicKey,
}

impl Rsa {
    pub fn new() -> Rsa {
        let mut rng = OsRng;
        let bits = 2048;
        let private_key = RSAPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
        let pub_key = RSAPublicKey::from(&private_key);
        Rsa {
            rng,
            private_key,
            public_key: pub_key,
        }
    }

    pub fn get_keypair(&self) -> Box<(RSAPrivateKey, RSAPublicKey)> {
        Box::new((self.private_key.clone(), self.public_key.clone()))
    }

    pub fn encrypt(&mut self, msg: &String) -> Vec<u8> {
        let message = msg.as_bytes();
        let padding = PaddingScheme::new_pkcs1v15_encrypt();
        let enc_data = self
            .public_key
            .encrypt(&mut self.rng, padding, &message[..])
            .expect("failed to encrypt");
        enc_data
    }

    pub fn decrypt(&self, msg: &Vec<u8>) -> String {
        let padding = PaddingScheme::new_pkcs1v15_encrypt();
        let dec_data = self
            .private_key
            .decrypt(padding, msg)
            .expect("failed to decrypt");
        String::from_utf8(dec_data).unwrap()
    }
}

#[cfg(test)]
mod rsa_tests {
    use super::*;

    #[test]
    fn rsa_test() {
        let mut rsa = Rsa::new();
        let data = String::from("test");
        println!("明文字符串：{}", data);
        println!("***************************************************************");
        let msg1 = rsa.encrypt(&data);
        println!("RSA加密： {:?}", msg1);
        println!("***************************************************************");
        let msg2 = rsa.decrypt(&msg1);
        println!("RSA解密： {:?}", msg2);
    }
}
