use std::{collections::HashMap, hash::Hash};

use chrono::Local;
use libp2p::PeerId;
use network::peer::Peer;

use crate::{
    common::get_request_hash,
    message::{ConsensusEndData, ConsensusStartData, Request, TestItem},
};

#[derive(Debug, Clone)]
// Store process data for consensus protocol
pub struct DataWarehouse {
    test_start_time: i64,

    // Throughput test data
    t_start_data: HashMap<String, ConsensusStartData>,
    t_end_data: HashMap<(PeerId, String), ConsensusEndData>,
    t_start_data_computing: HashMap<String, ConsensusStartData>,
    t_end_data_computing: HashMap<(PeerId, String), ConsensusEndData>,
    // Throughput results
    throughput_mid_results: HashMap<PeerId, u64>,
    throughput_results: HashMap<u64, (PeerId, u64)>,

    // Latency test data
    l_start_data: HashMap<String, ConsensusStartData>,
    l_end_data: HashMap<(PeerId, String), ConsensusEndData>,
    l_start_data_computing: HashMap<String, ConsensusStartData>,
    l_end_data_computing: HashMap<(PeerId, String), ConsensusEndData>,
    // Latency results
    latency_results: HashMap<PeerId, Vec<LatencyResult>>,

    // Scalability results
    scalability_mid_throughputs: HashMap<PeerId, u64>,
    scalability_mid_latencies: HashMap<PeerId, Vec<LatencyResult>>,
    scalability_results: HashMap<u16, HashMap<PeerId, ScalabilityResult>>,

    // security test data
    s_start_data: HashMap<String, ConsensusStartData>,
    s_end_data: HashMap<(PeerId, String), ConsensusEndData>,

    // Crash results
    crash_nodes: HashMap<u16, Vec<PeerId>>,
    crash_results: HashMap<u16, HashMap<PeerId, SecurityResult>>,
    // Malicious results
    malicious_results: HashMap<TestItem, HashMap<PeerId, SecurityResult>>

}

// Latency result
#[derive(Debug, Clone)]
pub struct LatencyResult {
    pub request: Request,
    pub latency: u64,
}

// Scalability result
#[derive(Debug, Clone)]
pub struct ScalabilityResult {
    pub throughput: u64,
    pub latency_results: Vec<LatencyResult>,
}

// Crash result
#[derive(Debug, Clone)]
pub struct SecurityResult(Vec<Request>);

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
            s_start_data: HashMap::new(),
            s_end_data: HashMap::new(),
            throughput_mid_results: HashMap::new(),
            throughput_results: HashMap::new(),
            latency_results: HashMap::new(),
            scalability_mid_throughputs: HashMap::new(),
            scalability_mid_latencies: HashMap::new(),
            scalability_results: HashMap::new(),
            crash_nodes: HashMap::new(),
            crash_results: HashMap::new(),
            malicious_results: HashMap::new(),
        }
    }

    pub fn print_throughput_results(&self) {
        println!("\n【Throughput Results】:");
        for (index, result) in &self.throughput_results {
            println!("【{:?}】 ==> {:?}", index, result);
        }
    }

    pub fn print_latency_results(&self) {
        println!("\n【Latency Results】:");
        for (peer_id, latency_vec) in &self.latency_results {
            println!("NodeID({:?}):", peer_id);
            for r in latency_vec {
                println!("{:?} ==> {}", r.request, r.latency);
            }
        }
    }

    pub fn print_scalability_results(&self) {
        println!("\n【Scalability Results】:");
        for (node_count, scalabiliy_results) in &self.scalability_results {
            println!("The number of consensus node: {}", node_count);
            for (peer_id, scalability_result) in scalabiliy_results {
                println!("NodeID({:?}) ==> {:?}", peer_id, scalability_result);
            }
        }
    }

    pub fn record_crash_node(&mut self, count: u16, peer_id: PeerId) {
        if 0 != self.crash_nodes.len() {
            let (_, last_one) = self.crash_nodes.iter().last().unwrap();
            let mut new_vec = last_one.clone();
            new_vec.push(peer_id);

            self.crash_nodes.insert(count, new_vec);
        } else {
            self.crash_nodes.insert(count, vec![peer_id]);
        }
    }

    pub fn set_test_start_time(&mut self, start_time: i64) {
        self.test_start_time = start_time;
    }

    pub fn store_consensus_start_data(&mut self, origin_peer_id: PeerId, data: ConsensusStartData) {
        let request_hash = get_request_hash(&data.request);
        self.t_start_data
            .insert(request_hash.clone(), data.clone());
        self.l_start_data
            .insert(request_hash, data);
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

        let data_computing = self.t_end_data_computing.clone();
        data_computing.iter().for_each(|(k, _)| {
            if let Some(_) = self.t_start_data_computing.get(&k.1) {
                self.update_request_count_with_peer(k.0);

                self.t_end_data_computing.remove(k);
            }
        });

        let index = self.throughput_results.len() as u64;
        self.throughput_results = self
            .throughput_mid_results
            .iter()
            .map(|(&peer_id, &count)| {
                let throughput =
                    (count as f64 / 0.001 * (current_time - self.test_start_time) as f64) as u64;
                (index, (peer_id, throughput))
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

        let data_computing = self.l_end_data_computing.clone();
        data_computing.iter().for_each(|(k, end_data)| {
            if let Some(start_data) = self.l_start_data_computing.get(&k.1) {
                let latency = (end_data.completed_time - start_data.start_time) as u64;
                let latency_result = LatencyResult {
                    request: start_data.request.clone(),
                    latency,
                };
                self.store_latency_result(k.0, latency_result);
                self.l_end_data_computing.remove(k);
            } else {
                let latency_result = LatencyResult {
                    request: end_data.request.clone(),
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

    fn compute_scalability_throughput(&mut self) {
        self.prepare_compute_throughput();
        let current_time = Local::now().timestamp_millis();

        // let current_time = 30;

        let data_computing = self.t_end_data_computing.clone();
        data_computing.iter().for_each(|(k, _)| {
            if let Some(_) = self.t_start_data_computing.get(&k.1) {
                self.update_request_count_with_peer(k.0);

                self.t_end_data_computing.remove(k);
            }
        });

        self.scalability_mid_throughputs = self
            .throughput_mid_results
            .iter()
            .map(|(&peer_id, &count)| {
                let throughput =
                    (count as f64 / 0.001 * (current_time - self.test_start_time) as f64) as u64;
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

    fn compute_scalability_latency(&mut self) {
        self.prepare_compute_latency();

        let data_computing = self.l_end_data_computing.clone();
        data_computing.iter().for_each(|(k, end_data)| {
            if let Some(start_data) = self.l_start_data_computing.get(&k.1) {
                let latency = (end_data.completed_time - start_data.start_time) as u64;
                let latency_result = LatencyResult {
                    request: start_data.request.clone(),
                    latency,
                };
                self.record_scalability_mid_latency(k.0, latency_result);
                self.l_end_data_computing.remove(k);
            } else {
                let latency_result = LatencyResult {
                    request: end_data.request.clone(),
                    latency: 0,
                };
                self.record_scalability_mid_latency(k.0, latency_result);
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

    pub fn compute_scalability(&mut self, node_count: u16) {
        self.compute_scalability_latency();
        self.compute_scalability_throughput();

        let mut scalablity_results: HashMap<PeerId, ScalabilityResult> = HashMap::new();
        let scalability_mid_throughputs = self.scalability_mid_throughputs.clone();
        scalability_mid_throughputs
            .iter()
            .for_each(|(&peer_id, &throughput)| {
                if let Some(latency_results) = self.scalability_mid_latencies.get(&peer_id) {
                    let scalabililty_result = ScalabilityResult {
                        throughput,
                        latency_results: latency_results.clone(),
                    };
                    scalablity_results.insert(peer_id, scalabililty_result);

                    self.scalability_mid_throughputs.remove(&peer_id);
                    self.scalability_mid_latencies.remove(&peer_id);
                } else {
                    let scalabililty_result = ScalabilityResult {
                        throughput,
                        latency_results: vec![],
                    };
                    scalablity_results.insert(peer_id, scalabililty_result);

                    self.scalability_mid_throughputs.remove(&peer_id);
                }
            });

        self.scalability_mid_latencies
            .iter()
            .for_each(|(&peer_id, latency_results)| {
                let scalabililty_result = ScalabilityResult {
                    throughput: 0,
                    latency_results: latency_results.clone(),
                };
                scalablity_results.insert(peer_id, scalabililty_result);
            });

        self.scalability_results
            .insert(node_count, scalablity_results);
    }

    pub fn test_crash(&mut self, crash_count: u16) {
        let mut crash_results = self.crash_results(crash_count);

        let data = self.s_end_data.clone();
        data.iter().for_each(|(k, end_data)| {
            if let Some(start_data) = self.s_start_data.get(&k.1) {
                // let crash_result = CrashResult {
                //     request_cmd: end_data.request.cmd.clone(),
                //     timestamp: end_data.completed_time as u64,
                // };
                DataWarehouse::store_crash_result(&mut crash_results, &k.0, end_data.request.clone());
                self.s_end_data.remove(k);
            } else {
                // let crash_result = CrashResult {
                //     request_cmd: end_data.request.cmd.clone(),
                //     timestamp: 0,
                // };
                // DataWarehouse::store_crash_result(&mut crash_results, &k.0, crash_result);
            };
        });

        // let c_end_data_clone = self.c_end_data.clone();
        // let c_end_data_clone_iter = c_end_data_clone.iter();
        // let c_end_data = &mut self.c_end_data;
        // c_end_data_clone_iter.for_each(|(k, v)| {
        //     let crash_result = CrashResult {
        //         request_cmd: v.request.cmd.clone(),
        //         timestamp: v.completed_time as u64,
        //     };
        //     DataWarehouse::store_crash_result(&mut crash_results, &k.0, crash_result);
        //     c_end_data.remove(k);
        // });

        self.crash_results.insert(crash_count, crash_results);
    }

    pub fn crash_results(&mut self, crash_count: u16) -> HashMap<PeerId, SecurityResult> {
        if let Some(crash_resuls) = self.crash_results.get(&crash_count) {
            crash_resuls.clone()
        } else {
            let crash_results: HashMap<PeerId, SecurityResult> = HashMap::new();
            self.crash_results.insert(crash_count, crash_results);
            self.crash_results.get(&crash_count).unwrap().clone()
        }
    }

    pub fn test_malicious(&mut self, test_item: TestItem) {
        
    }

    pub fn store_crash_result(
        crash_results: &mut HashMap<PeerId, SecurityResult>,
        peer_id: &PeerId,
        crash_result: Request,
    ) {
        if let Some(crash_result_vec) = crash_results.get_mut(&peer_id) {
            crash_result_vec.0.push(crash_result);
        } else {
            let r = SecurityResult(vec![crash_result]);
            crash_results.insert(*peer_id, r);
        }
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

    pub fn record_scalability_mid_latency(
        &mut self,
        peer_id: PeerId,
        latency_result: LatencyResult,
    ) {
        if let Some(latency_result_vec) = self.scalability_mid_latencies.get_mut(&peer_id) {
            latency_result_vec.push(latency_result);
        } else {
            let new_vec = vec![latency_result];
            self.scalability_mid_latencies.insert(peer_id, new_vec);
        }
    }

    pub fn reset(&mut self) {
        self.t_start_data.clear();
        self.t_end_data.clear();
        self.t_start_data_computing.clear();
        self.t_end_data_computing.clear();
        self.l_start_data.clear();
        self.l_end_data.clear();
        self.l_start_data_computing.clear();
        self.l_end_data_computing.clear();
        self.throughput_mid_results.clear();
    }
}

#[cfg(test)]
pub mod data_warehouse_test {
    use core::time;

    use chrono::Local;
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
            flag: false,
            timestamp: Local::now().timestamp_millis(),
        };
        let request_2 = Request {
            cmd: "request_2".to_string(),
            flag: false,
            timestamp: Local::now().timestamp_millis(),
        };
        let request_3 = Request {
            cmd: "request_3".to_string(),
            flag: false,
            timestamp: Local::now().timestamp_millis(),
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

        println!(
            "{}, {}",
            end_time_millis - current_time_millis,
            end_time_sec - current_time_sec
        );

        println!("===================================================================");
        println!("                                                                   ");
        println!(
            "
            
            __        __   _                            _                         
            \\ \\      / /__| | ___ ___  _ __ ___   ___  | |_ ___    _   _ ___  ___ 
             \\ \\ /\\ / / _ \\ |/ __/ _ \\| '_ ` _ \\ / _ \\ | __/ _ \\  | | | / __|/ _ \\
              \\ V  V /  __/ | (_| (_) | | | | | |  __/ | || (_) | | |_| \\__ \\  __/
               \\_/\\_/ \\___|_|\\___\\___/|_| |_| |_|\\___|  \\__\\___/   \\__,_|___/\\___|
                                                                                  
             ____  _____ _____ ____  _                             _              
            | __ )|  ___|_   _|  _ \\(_) __ _  __ _ _ __   ___  ___(_)___          
            |  _ \\| |_    | | | | | | |/ _` |/ _` | '_ \\ / _ \\/ __| / __|         
            | |_) |  _|   | | | |_| | | (_| | (_| | | | | (_) \\__ \\ \\__ \\         
            |____/|_|     |_| |____/|_|\\__,_|\\__, |_| |_|\\___/|___/_|___/         
                                             |___/                               
            
            "
        );
        println!("Welcome to use BFTDiagnosis！");
        println!("                                                                   ");
        println!("===================================================================");
    }
}
