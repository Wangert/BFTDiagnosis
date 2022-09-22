use crypto::{digest::Digest, sha3::Sha3};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub fn block_serialize<T: ?Sized>(value: &T) -> Vec<u8>
where
    T: Serialize,
{
    let seialized = bincode::serialize(value).unwrap();
    seialized
}

pub fn block_deserialize<'a, T>(bytes: &'a [u8]) -> T
where
    T: Deserialize<'a>,
{
    let deserialized = bincode::deserialize(bytes).unwrap();
    deserialized
}

pub fn deserialize_for_bytes<'a, T>(bytes: &'a [u8]) -> T
where
    T: Deserialize<'a>,
{
    bincode::deserialize(bytes).unwrap()
}



pub fn serialize_into_bytes<T: ?Sized>(value: &T) -> Vec<u8>
where
    T: Serialize,
{
    let seialized = bincode::serialize(value).unwrap();
    seialized
}

pub fn serialize_into_json_bytes<T: ?Sized>(value: &T) -> String
where
    T: Serialize,
{
    let json_bytes = serde_json::to_string(value).unwrap();
    json_bytes
}

pub fn deserialize_for_json_bytes<'a, T: ?Sized>(json_bytes: &'a [u8]) -> T 
where
    T: Deserialize<'a>,
{
    let deserialized = serde_json::from_slice(json_bytes).unwrap();
    deserialized
}

pub fn get_hash_u8_vec(value: &[u8], mut out: &mut [u8]) {
    let mut hasher = Sha3::sha3_256();
    hasher.input(value);
    hasher.result(&mut out);
}

pub fn get_hash_str(value: &[u8]) -> String {
    let mut hasher = Sha3::sha3_256();
    hasher.input(value);
    hasher.result_str()
}

#[cfg(test)]
mod coder_test {
    use std::{collections::HashMap, borrow::BorrowMut};

    use serde::{Deserialize, Serialize};
    use serde_json::{Value, Map, json};

    use crate::coder::{get_hash_u8_vec, get_hash_str, deserialize_for_json_bytes};

    use super::serialize_into_json_bytes;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct PreCommit {
        pub view_num: u64,
        // pub block: Block,
        pub justify: Option<QC>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    pub struct QC {
        pub msg_type: u8,
        pub view_num: u64,
        pub block: String,
        // pub signature: Option<Signature>,
    }

    #[test]
    fn hash_works() {
        let s = "wangjitao".as_bytes();
        let mut out: [u8;64] = ['0' as u8;64];
        get_hash_u8_vec(s, &mut out);
        println!("out: {:?}", &out);
        
        let hash_str = get_hash_str(s);
        println!("str: {:?}", hash_str);
    }

    #[test]
    fn json_bytes_works() {

        let qc = QC {
            msg_type: 2,
            view_num: 45,
            block: "block".to_string(),
        };
        let pc = PreCommit {
            view_num: 100,
            justify: Some(qc),
        };

        let json_bytes = serialize_into_json_bytes(&pc);
        println!("first serialize: {:?}", json_bytes);

        let de: PreCommit = deserialize_for_json_bytes(json_bytes.as_bytes());

        println!("first deserialize: {:?}", de);


        let parsed: Value = serde_json::from_slice(json_bytes.as_bytes()).unwrap();
        let mut obj: Map<String, Value> = parsed.as_object().unwrap().clone();

        println!("parse map: {:?}", obj);

        let obj_clone = obj.clone();
        for (k, v )in obj_clone {
            match v {
                Value::Null => todo!(),
                Value::Bool(_) => todo!(),
                Value::Number(_) => {
                    obj.insert(k, json!(999));
                },
                Value::String(_) => {
                    obj.insert(k, json!("wangjitao".to_string()));
                },
                Value::Array(_) => todo!(),
                Value::Object(o) => {
                    let mut o_clone = o.clone();
                    o.iter().for_each(|(k, v)| {
                        match v {
                            Value::Null => todo!(),
                            Value::Bool(_) => todo!(),
                            Value::Number(_) => {},
                            Value::String(_) => {
                                o_clone.borrow_mut().insert(k.clone(), json!("block_clone".to_string()));
                            },
                            Value::Array(_) => todo!(),
                            Value::Object(_) => todo!(),
                        }
                    });

                    obj.insert(k, json!(o_clone));
                },
            }
        }

        let map_json_bytes = serialize_into_json_bytes(&obj);
        println!("after motify json: {:?}", map_json_bytes);

        let final_de: PreCommit = deserialize_for_json_bytes(map_json_bytes.as_bytes());
        println!("after motify precommit: {:?}", final_de);

    }
}