use libp2p::{multiaddr::Protocol, Multiaddr};
use serde_json::{json, Map, Value};
use trees::Tree;

pub fn into_ip4_tcp_multiaddr(ip_addr: &str, port: u16) -> Multiaddr {
    let mut prefix = String::from("/ip4/");
    prefix.push_str(ip_addr);
    let mut addr: Multiaddr = prefix.parse().unwrap();
    addr.push(Protocol::Tcp(port));

    addr
}

pub fn map_into_string_tree(root_value: &str, map: Map<String, Value>) -> (Tree<String>, Vec<Vec<String>>) {
    let mut tree = Tree::new(root_value.to_string());
    let mut field_set: Vec<Vec<String>> = Vec::new();
    for (k, v) in map {
        match v {
            Value::Object(sub_map) => {
                let (t, h_set) = map_into_string_tree(&k, sub_map);
                h_set.iter().for_each(|v| {
                    let mut new_vec = vec![k.clone()];
                    let mut v_clone = v.clone();
                    new_vec.append(&mut v_clone);
                    field_set.push(new_vec);
                });

                tree.push_back(t);
            },
            _ => {
                field_set.push(vec![k.clone()]);
                tree.push_back(Tree::new(k));
            },
        };
    }

    (tree, field_set)
}

pub fn motify_map_value_with_field(
    map: Map<String, Value>,
    field: Vec<String>,
) -> Map<String, Value> {
    println!("field:{:?}",field.clone());
    let f = field.get(0).unwrap();
    println!("f: {:?}",f.clone());
    println!("map:{:?}",map.clone());
    //let value = map.get(f).unwrap().clone();
    let value = map.get(f).unwrap().clone();
    println!("value:{:?}",value.clone());
    // let mut final_map = Map::new();
    let mut field_clone = field.clone();

    if 1 == field_clone.len() {
        let new_value = match value {
            Value::Null => {
                json!(null)
            }
            Value::Bool(b) => {
                json!(!b)
            }
            Value::Number(n) => {
                json!(n.as_f64().unwrap() + 99 as f64)
            }
            Value::String(_) => {
                json!("ErrorString")
            }
            Value::Array(_) => {
                json!(["E", "R", "R", "O", "R"])
            }
            Value::Object(o) => {
                json!(o)
            }
        };

        let mut new_map = map.clone();
        new_map.insert(f.to_string(), new_value);
        return new_map;
    } else {
        if let Value::Object(next_map) = value {
            field_clone.remove(0);
            let new_value = json!(motify_map_value_with_field(next_map, field_clone));
            let mut new_map = map.clone();
            new_map.insert(f.to_string(), new_value);
            return new_map;
        } else {
            panic!("Motify Map Value With Field Error!!!");
        }
    }

    // return final_map;
}

#[cfg(test)]
pub mod parse_tests {
    use serde::{Deserialize, Serialize};
    use serde_json::{Map, Value};

    use crate::{
        coder::{deserialize_for_json_bytes, serialize_into_json_str},
        parse::map_into_string_tree,
    };

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
    fn map_into_string_tree_works() {
        let qc = QC {
            msg_type: 2,
            view_num: 45,
            block: "block".to_string(),
        };
        let pc = PreCommit {
            view_num: 100,
            justify: Some(qc),
        };

        let json_bytes = serialize_into_json_str(&pc);
        println!("first serialize: {:?}", json_bytes);

        let de: PreCommit = deserialize_for_json_bytes(json_bytes.as_bytes());

        println!("first deserialize: {:?}", de);

        let parsed: Value = serde_json::from_slice(json_bytes.as_bytes()).unwrap();
        let map: Map<String, Value> = parsed.as_object().unwrap().clone();

        println!("parse map: {:?}", map);

        let tree = map_into_string_tree("1", map);

        println!("{:#?}", tree);
    }
}
