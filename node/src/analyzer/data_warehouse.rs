use std::collections::HashMap;

use libp2p::PeerId;

use crate::{
    common::get_request_hash,
    message::{ConsensusEndData, ConsensusStartData, Request},
};

// Store process data for consensus protocol
pub struct DataWarehouse {
    t_start_data: HashMap<PeerId, Vec<ConsensusStartData>>,
    t_end_data: HashMap<PeerId, Vec<ConsensusEndData>>,

    l_start_data: HashMap<(PeerId, String), ConsensusStartData>,
    l_end_data: HashMap<(PeerId, String), ConsensusEndData>,
    l_start_data_computing: HashMap<(PeerId, String), ConsensusStartData>,
    l_end_data_computing: HashMap<(PeerId, String), ConsensusEndData>,

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
            t_start_data: HashMap::new(),
            t_end_data: HashMap::new(),
            l_start_data: HashMap::new(),
            l_end_data: HashMap::new(),
            l_start_data_computing: HashMap::new(),
            l_end_data_computing: HashMap::new(),
            throughput_results: HashMap::new(),
            latency_results: HashMap::new(),
            scalability_results: HashMap::new(),
        }
    }

    pub fn store_consensus_start_data(&mut self, origin_peer_id: PeerId, data: ConsensusStartData) {
        if let Some(start_data_vec) = self.t_start_data.get_mut(&origin_peer_id) {
            start_data_vec.push(data.clone());
        } else {
            let new_vec = vec![data.clone()];
            self.t_start_data.insert(origin_peer_id, new_vec);
        }

        let request_hash = get_request_hash(&data.request);
        self.l_start_data
            .insert((origin_peer_id, request_hash), data);
    }

    pub fn store_consensus_end_data(&mut self, origin_peer_id: PeerId, data: ConsensusEndData) {
        if let Some(end_data_vec) = self.t_end_data.get_mut(&origin_peer_id) {
            end_data_vec.push(data.clone());
        } else {
            let new_vec = vec![data.clone()];
            self.t_end_data.insert(origin_peer_id, new_vec);
        }

        let request_hash = get_request_hash(&data.request);

        self.l_end_data.insert((origin_peer_id, request_hash), data);
    }

    pub fn prepare_compute_latency(&mut self) {
        self.l_start_data_computing = self.l_start_data.clone();
        self.l_start_data.clear();

        self.l_end_data_computing = self.l_end_data.clone();
        self.l_end_data.clear();
    }

    pub fn compute_throughput(&mut self) {}

    pub fn compute_latency(&mut self) {
        self.prepare_compute_latency();

        let data_computing = self.l_start_data_computing.clone();
        data_computing.iter().for_each(|(k, start_data)| {
            if let Some(end_data) = self.l_end_data_computing.get(k) {
                let latency = end_data.completed_time - start_data.start_time;
                let latency_result = LatencyResult {
                    request: start_data.request.clone(),
                    latency,
                };
                self.store_latency_result(k.0, latency_result);
            } else {
                let latency_result = LatencyResult {
                    request: start_data.request.clone(),
                    latency: 0,
                };
                self.store_latency_result(k.0, latency_result);
            };
        });
    }

    pub fn store_throughput_result(&mut self, peer_id: PeerId, throughput: u64) {}

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
        data_warehouse.compute_latency();
        println!("After Compute Latency:");
        println!("{:#?}", data_warehouse.latency_results);
    }
}
