use crypto::{sha3::Sha3, digest::Digest};
use serde::{Serialize, Deserialize};



pub fn block_serialize<T: ?Sized>(value: &T) -> Vec<u8> 
    where T: Serialize,
{
    let seialized = bincode::serialize(value).unwrap();
    seialized
}

pub fn block_deserialize<'a, T>(bytes: &'a[u8]) -> T 
    where T: Deserialize<'a>,
{
    let deserialized = bincode::deserialize(bytes).unwrap();
    deserialized
}

pub fn deserialize_for_bytes<'a, T>(bytes: &'a [u8]) -> T 
where T: Deserialize<'a>
{
    bincode::deserialize(bytes).unwrap()
}

pub fn get_hash(value: &[u8], mut out: &mut [u8]) {
    let mut hasher = Sha3::sha3_256();
    hasher.input(value);
    hasher.result(&mut out);
}

