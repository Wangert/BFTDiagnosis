use std::{collections::HashMap, hash::Hash};

use chrono::Local;
use libp2p::PeerId;
use network::peer::Peer;

use crate::{
    common::get_request_hash,
    message::{ConsensusEndData, ConsensusStartData, Request},
};

// Store process data for consensus protocol
pub struct DataWarehouse {
    test_start_time: i64,

    t_start_data: HashMap<(PeerId, String), ConsensusStartData>,
    t_end_data: HashMap<(PeerId, String), ConsensusEndData>,
    t_start_data_computing: HashMap<(PeerId, String), ConsensusStartData>,
    t_end_data_computing: HashMap<(PeerId, String), ConsensusEndData>,

    l_start_data: HashMap<(PeerId, String), ConsensusStartData>,
    l_end_data: HashMap<(PeerId, String), ConsensusEndData>,
    l_start_data_computing: HashMap<(PeerId, String), ConsensusStartData>,
    l_end_data_computing: HashMap<(PeerId, String), ConsensusEndData>,

    throughput_mid_results: HashMap<PeerId, u64>,
    throughput_results: HashMap<PeerId, u64>,
    latency_results: HashMap<PeerId, Vec<LatencyResult>>,
    scalability_results: HashMap<(PeerId, u16), ScalabilityResult>,
}

#[derive(Debug)]
pub struct LatencyResult {
    pub request: Request,
    pub latency: u64,
}

pub struct ScalabilityResult {
    pub throughput: u64,
    pub latency_results: Vec<LatencyResult>,
}

impl DataWarehouse {
    pub fn new() -> Self {
        Self {
            test_start_time: 0,
            t_start_data: HashMap::new(),
            t_end_data: HashMap::new(),
            t_start_data_computing: HashMap::new(),
            t_end_data_computing: HashMap::new(),
            l_start_data: HashMap::new(),
            l_end_data: HashMap::new(),
            l_start_data_computing: HashMap::new(),
            l_end_data_computing: HashMap::new(),
            throughput_mid_results: HashMap::new(),
            throughput_results: HashMap::new(),
            latency_results: HashMap::new(),
            scalability_results: HashMap::new(),
        }
    }

    pub fn set_test_start_time(&mut self, start_time: i64) {
        self.test_start_time = start_time;
    }

    pub fn store_consensus_start_data(&mut self, origin_peer_id: PeerId, data: ConsensusStartData) {
        let request_hash = get_request_hash(&data.request);
        self.t_start_data
            .insert((origin_peer_id, request_hash.clone()), data.clone());
        self.l_start_data
            .insert((origin_peer_id, request_hash), data);
    }

    pub fn store_consensus_end_data(&mut self, origin_peer_id: PeerId, data: ConsensusEndData) {
        let request_hash = get_request_hash(&data.request);
        self.t_end_data
            .insert((origin_peer_id, request_hash.clone()), data.clone());
        self.l_end_data.insert((origin_peer_id, request_hash), data);
    }

    pub fn prepare_compute_throughput(&mut self) {
        self.t_start_data_computing = self.t_start_data.clone();
        self.t_start_data.clear();

        self.t_end_data_computing = self.t_end_data.clone();
        self.t_end_data.clear();
    }

    pub fn prepare_compute_latency(&mut self) {
        self.l_start_data_computing = self.l_start_data.clone();
        self.l_start_data.clear();

        self.l_end_data_computing = self.l_end_data.clone();
        self.l_end_data.clear();
    }

    pub fn compute_throughput(&mut self) {
        self.prepare_compute_throughput();
        let current_time = Local::now().timestamp_millis();

        
        // let current_time = 30;

        let data_computing = self.t_start_data_computing.clone();
        data_computing.iter().for_each(|(k, _)| {
            if let Some(_) = self.t_end_data_computing.get(k) {
                self.update_request_count_with_peer(k.0);

                self.t_start_data_computing.remove(k);
                self.t_end_data_computing.remove(k);
            }
        });

        self.throughput_results = self
            .throughput_mid_results
            .iter()
            .map(|(&peer_id, &count)| {
                let throughput = (count as f64 / 0.001 * (current_time - self.test_start_time) as f64) as u64;
                (peer_id, throughput)
            })
            .collect();

        // The remaining data is rewritten to t_start_data and t_end_data
        self.t_start_data_computing.iter().for_each(|(k, v)| {
            self.t_start_data.insert(k.clone(), v.clone());
        });
        self.t_end_data_computing.iter().for_each(|(k, v)| {
            self.t_end_data.insert(k.clone(), v.clone());
        });

        self.t_start_data_computing.clear();
        self.t_end_data_computing.clear();
    }

    pub fn compute_latency(&mut self) {
        self.prepare_compute_latency();

        let data_computing = self.l_start_data_computing.clone();
        data_computing.iter().for_each(|(k, start_data)| {
            if let Some(end_data) = self.l_end_data_computing.get(k) {
                let latency = (end_data.completed_time - start_data.start_time) as u64;
                let latency_result = LatencyResult {
                    request: start_data.request.clone(),
                    latency,
                };
                self.store_latency_result(k.0, latency_result);
                self.l_start_data_computing.remove(k);
                self.l_end_data_computing.remove(k);
            } else {
                let latency_result = LatencyResult {
                    request: start_data.request.clone(),
                    latency: 0,
                };
                self.store_latency_result(k.0, latency_result);
            };
        });

        // The remaining data is rewritten to l_start_data and l_end_data
        self.l_start_data_computing.iter().for_each(|(k, v)| {
            self.l_start_data.insert(k.clone(), v.clone());
        });
        self.l_end_data_computing.iter().for_each(|(k, v)| {
            self.l_end_data.insert(k.clone(), v.clone());
        });

        self.l_start_data_computing.clear();
        self.l_end_data_computing.clear();
    }

    pub fn update_request_count_with_peer(&mut self, peer_id: PeerId) {
        if let Some(&count) = self.throughput_mid_results.get(&peer_id) {
            self.throughput_mid_results.insert(peer_id, count + 1);
        } else {
            self.throughput_mid_results.insert(peer_id, 1);
        }
    }

    pub fn store_latency_result(&mut self, peer_id: PeerId, latency_result: LatencyResult) {
        if let Some(latency_result_vec) = self.latency_results.get_mut(&peer_id) {
            latency_result_vec.push(latency_result);
        } else {
            let new_vec = vec![latency_result];
            self.latency_results.insert(peer_id, new_vec);
        }
    }
}

#[cfg(test)]
pub mod data_warehouse_test {
    use core::time;

    use chrono::{Local, DateTime};
    use libp2p::PeerId;

    use crate::message::{ConsensusEndData, ConsensusStartData, Request};

    use super::DataWarehouse;

    #[test]
    pub fn compute_latency_works() {
        let peer_1 = PeerId::random();
        let peer_2 = PeerId::random();

        println!("Peer_1:{:?}", peer_1);
        println!("Peer_2:{:?}", peer_2);

        let request_1 = Request {
            cmd: "request_1".to_string(),
        };
        let request_2 = Request {
            cmd: "request_2".to_string(),
        };
        let request_3 = Request {
            cmd: "request_3".to_string(),
        };

        let consensus_start_data_11 = ConsensusStartData {
            request: request_1.clone(),
            start_time: 11,
        };
        let consensus_start_data_12 = ConsensusStartData {
            request: request_2.clone(),
            start_time: 21,
        };
        let consensus_start_data_13 = ConsensusStartData {
            request: request_3.clone(),
            start_time: 31,
        };
        let consensus_start_data_21 = ConsensusStartData {
            request: request_1.clone(),
            start_time: 12,
        };
        let consensus_start_data_22 = ConsensusStartData {
            request: request_2.clone(),
            start_time: 22,
        };
        let consensus_start_data_23 = ConsensusStartData {
            request: request_3.clone(),
            start_time: 32,
        };

        let consensus_end_data_11 = ConsensusEndData {
            request: request_1.clone(),
            completed_time: 12,
        };
        let consensus_end_data_12 = ConsensusEndData {
            request: request_2.clone(),
            completed_time: 23,
        };
        let consensus_end_data_13 = ConsensusEndData {
            request: request_3.clone(),
            completed_time: 34,
        };
        let consensus_end_data_21 = ConsensusEndData {
            request: request_1.clone(),
            completed_time: 14,
        };
        let consensus_end_data_22 = ConsensusEndData {
            request: request_2.clone(),
            completed_time: 26,
        };
        let consensus_end_data_23 = ConsensusEndData {
            request: request_3.clone(),
            completed_time: 38,
        };

        let mut data_warehouse = DataWarehouse::new();
        data_warehouse.store_consensus_start_data(peer_1, consensus_start_data_11);
        data_warehouse.store_consensus_start_data(peer_1, consensus_start_data_12);
        data_warehouse.store_consensus_start_data(peer_1, consensus_start_data_13);
        data_warehouse.store_consensus_start_data(peer_2, consensus_start_data_21);
        data_warehouse.store_consensus_start_data(peer_2, consensus_start_data_22);
        data_warehouse.store_consensus_start_data(peer_2, consensus_start_data_23);
        data_warehouse.store_consensus_end_data(peer_1, consensus_end_data_11);
        data_warehouse.store_consensus_end_data(peer_1, consensus_end_data_12);
        data_warehouse.store_consensus_end_data(peer_1, consensus_end_data_13);
        data_warehouse.store_consensus_end_data(peer_2, consensus_end_data_21);
        data_warehouse.store_consensus_end_data(peer_2, consensus_end_data_22);
        data_warehouse.store_consensus_end_data(peer_2, consensus_end_data_23);

        println!("Before Compute Latency:");
        println!("{:#?}", data_warehouse.latency_results);
        println!("Before Compute Throughput:");
        println!("{:#?}", data_warehouse.throughput_mid_results);
        println!("{:#?}", data_warehouse.throughput_results);
        data_warehouse.compute_latency();
        data_warehouse.compute_throughput();
        println!("After Compute Latency:");
        println!("{:#?}", data_warehouse.latency_results);
        println!("After Compute Throughput:");
        println!("{:#?}", data_warehouse.throughput_mid_results);
        println!("{:#?}", data_warehouse.throughput_results);

        println!("{:#?}", data_warehouse.l_start_data_computing);
        
    }

    #[test]
    pub fn timestamp_works() {
        let current_time_millis = Local::now().timestamp_millis();
        let current_time_sec = Local::now().timestamp();

        std::thread::sleep(time::Duration::from_secs(3));

        let end_time_millis = Local::now().timestamp_millis();
        let end_time_sec = Local::now().timestamp();

        println!("{}, {}", end_time_millis - current_time_millis, end_time_sec - current_time_sec);
    }
}
