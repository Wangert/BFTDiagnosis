pub mod behaviour;
pub mod basic_consensus_node;
pub mod analyzer;
pub mod message;
pub mod common;
pub mod controller;
pub mod config;

pub mod example_consensus_node;

#[cfg(test)]
pub mod node_test {
    use std::collections::HashMap;

    #[test]
    fn iter_it_works() {
        let mut test_map: HashMap<u64, u64> = HashMap::new();
        test_map.insert(1, 1);
        test_map.insert(2, 2);
        test_map.insert(3, 3);

        println!("{:?}", &test_map);

        let mut iter_test = test_map.iter();
        let mut iter_take = test_map.iter().take(1);


        println!("{:?}", &iter_test.next());
        println!("{:?}", &test_map);
        println!("{:?}", &iter_take.next());
        println!("{:?}", &test_map);
    }
}