use crypto::{sha3::Sha3, digest::Digest};
use serde::{Serialize, Deserialize};



// Generate SHA256 Hash String
pub fn get_sha256(value: &[u8]) -> String {
    let mut hasher = Sha3::sha3_256();
    hasher.input(value);
    hasher.result_str()
}

pub fn serialize_to_bytes<T: ?Sized>(value: &T) -> Vec<u8>
where T: Serialize,
{
   bincode::serialize(value).unwrap()
}

pub fn deserialize_for_bytes<'a, T>(bytes: &'a [u8]) -> T 
where T: Deserialize<'a>
{
    bincode::deserialize(bytes).unwrap()
}