use std::{collections::HashMap, hash::Hash};

use chrono::Local;
use libp2p::PeerId;
use mysql::{params, prelude::Queryable, PooledConn};
use network::peer::Peer;
use storage::mysql_db::MysqlDB;

use crate::{
    common::get_request_hash,
    message::{ConsensusEndData, ConsensusStartData, Request, TestItem},
};

#[derive(Debug, Clone)]
// Store process data for consensus protocol
pub struct DataWarehouse {
    test_start_time: u64,

    mysql_db: MysqlDB,

    // Throughput test data
    t_start_data: HashMap<String, ConsensusStartData>,
    t_end_data: HashMap<(PeerId, String), ConsensusEndData>,
    t_start_data_computing: HashMap<String, ConsensusStartData>,
    t_end_data_computing: HashMap<(PeerId, String), ConsensusEndData>,
    // Throughput results
    throughput_mid_results: HashMap<PeerId, u64>,
    throughput_results: HashMap<(u64, PeerId), u64>,

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
    scalability_throughput_results: HashMap<TestItem, Vec<(u64, PeerId, u64)>>,
    scalability_latency_results: HashMap<TestItem, Vec<(PeerId, Request, u64)>>,
    scalability_results: HashMap<u16, HashMap<PeerId, ScalabilityResult>>,

    // security test data
    s_start_data: HashMap<String, ConsensusStartData>,
    s_end_data: HashMap<(PeerId, String), ConsensusEndData>,

    // Crash results
    crash_nodes: HashMap<u16, Vec<PeerId>>,
    crash_results: HashMap<u16, HashMap<PeerId, SecurityResult>>,
    // Malicious results
    malicious_results: HashMap<TestItem, HashMap<PeerId, SecurityResult>>,

    internal_round: u64,
    throughput_internal: u64,
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
    pub fn new(url: &str) -> Self {
        Self {
            test_start_time: 0,
            mysql_db: MysqlDB::new(url),
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
            scalability_throughput_results: HashMap::new(),
            scalability_latency_results: HashMap::new(),
            scalability_results: HashMap::new(),
            crash_nodes: HashMap::new(),
            crash_results: HashMap::new(),
            malicious_results: HashMap::new(),
            internal_round: 0,
            throughput_internal: 0,
        }
    }

    pub fn print_throughput_results(&mut self) {
        println!("\n");
        println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");
        println!("@@@@@@@@  Throughput Results   @@@@@@@@");
        println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");

        let mut start = 0;
        loop {
            let results = self.query_throughput_results(start, 100);
            results.iter().for_each(|(round, peer_id, throughput)| {
                println!("============");
                println!("【{:?}】({:?}) ==> {:?}", round, peer_id, throughput);
            });

            let count = results.len() as u64;
            if 100 > count {
                println!("Total count: {}", start + count);
                break;
            }

            start += 100;
        }

        // for (index, result) in &self.throughput_results {
        //     println!("============");
        //     println!("【{:?}】 ==> {:?}", index, result);
        // }
    }

    pub fn print_latency_results(&mut self) {
        println!("\n");
        println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");
        println!("@@@@@@@@    Latency Results    @@@@@@@@");
        println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");

        let mut start = 0;
        loop {
            let results = self.query_latency_results(start, 100);
            results.iter().for_each(|(peer_id, request, latency)| {
                println!("============");
                println!("NodeID({:?}):", peer_id);
                println!("{:?} ==> {:?}", request, latency);
            });

            let count = results.len() as u64;
            if 100 > results.len() {
                println!("Total count: {}", start + count);
                break;
            }

            start += 100;
        }
        // for (peer_id, latency_vec) in &self.latency_results {
        //     println!("============");
        //     println!("NodeID({:?}):", peer_id);
        //     for r in latency_vec {
        //         println!("{:?} ==> {}", r.request, r.latency);
        //         println!("-------------");
        //     }
        // }
    }

    pub fn print_scalability_throughput_results(&mut self) {
        println!("\n");
        println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");
        println!("@@@  Scalability Throughput Results @@@");
        println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");

        let items = self.query_scalability_items();
        items.iter().for_each(|item| {
            println!("***************************");
            println!("{}", item);
            println!("***************************");
            let mut start = 0;
            loop {
                let results = self.query_scalability_throughput_results(&item, start, 100);
                results.iter().for_each(|(round, peer_id, throughput)| {
                    println!("============");
                    println!("【{:?}】({:?}) ==> {:?}", round, peer_id, throughput);
                });

                let count = results.len() as u64;
                if 100 > count {
                    println!("Total count: {}", start + count);
                    break;
                }

                start += 100;
            }
        });
    }

    pub fn print_scalability_latency_results(&mut self) {
        println!("\n");
        println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");
        println!("@@@@  Scalability Latency Results  @@@@");
        println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");

        let items = self.query_scalability_items();
        items.iter().for_each(|item| {
            println!("***************************");
            println!("{}", item);
            println!("***************************");
            let mut start = 0;
            loop {
                let results = self.query_scalability_latency_results(&item, start, 100);
                results.iter().for_each(|(peer_id, request, latency)| {
                    println!("============");
                    println!("NodeID({:?}):", peer_id);
                    println!("{:?} ==> {:?}", request, latency);
                });

                let count = results.len() as u64;
                if 100 > results.len() {
                    println!("Total count: {}", start + count);
                    break;
                }

                start += 100;
            }
        });
    }

    pub fn print_crash_results(&self) {
        println!("\n");
        println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");
        println!("@@@@@@@@     Crash Results     @@@@@@@@");
        println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");

        for (node_count, crash_results) in &self.crash_results {
            println!("============");
            println!("The number of consensus node: {}", node_count);
            for (peer_id, crash_result) in crash_results {
                println!("NodeID({:?}) ==> {:?}", peer_id, crash_result);
                println!("-------------");
            }
        }
    }

    pub fn print_malicious_results(&self) {
        println!("\n");
        println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");
        println!("@@@@@@@@   Malicious Results   @@@@@@@@");
        println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");

        for (item, malicious_results) in &self.malicious_results {
            println!("============");
            println!("Test Item: {:?}", item);
            for (peer_id, malicious_result) in malicious_results {
                println!("NodeID({:?}) ==> {:?}", peer_id, malicious_result);
                println!("-------------");
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

    pub fn set_test_start_time(&mut self, start_time: u64) {
        self.test_start_time = start_time;
    }

    pub fn set_throughput_internal(&mut self, internal: u64) {
        self.throughput_internal = internal;
    }

    pub fn store_consensus_start_data(&mut self, origin_peer_id: PeerId, data: ConsensusStartData) {
        let request_hash = get_request_hash(&data.request);
        self.t_start_data.insert(request_hash.clone(), data.clone());
        self.l_start_data.insert(request_hash.clone(), data.clone());
        self.s_start_data.insert(request_hash, data);
    }

    pub fn store_consensus_end_data(&mut self, origin_peer_id: PeerId, data: ConsensusEndData) {
        let request_hash = get_request_hash(&data.request);
        self.t_end_data
            .insert((origin_peer_id, request_hash.clone()), data.clone());
        self.l_end_data.insert((origin_peer_id, request_hash.clone()), data.clone());
        self.s_end_data.insert((origin_peer_id, request_hash), data);
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
        // let current_time = Local::now().timestamp_millis() as u64;

        // let current_time = 30;

        let data_computing = self.t_end_data_computing.clone();
        println!("throughout:{:?}",data_computing.clone());
        data_computing.iter().for_each(|(k, _)| {
            if let Some(_) = self.t_start_data_computing.get(&k.1) {
                self.update_request_count_with_peer(k.0);

                self.t_end_data_computing.remove(k);
            }
        });
        println!("self.throughput_mid_results:{:?}",self.throughput_mid_results.clone());
        let index = self.internal_round;
        self.internal_round += 1;
        let throughput_mid_results = self.throughput_mid_results.clone();
        throughput_mid_results
            .iter()
            .for_each(|(&peer_id, &count)| {
                println!(
                    "Count:{}; Round:{}; Internal:{}",
                    count, self.internal_round, self.throughput_internal
                );
                let throughput = (count as f64
                    / (0.001 * (self.internal_round * self.throughput_internal) as f64))
                    as u64;
                self.insert_throughput_result(index, &peer_id, throughput);
                // self.throughput_results.insert((index, peer_id), throughput);
            });

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
                println!("latency{},{}",end_data.completed_time.clone(),start_data.start_time);
                // let latency = (end_data.completed_time as u64 - start_data.start_time as u64);
                let latency = end_data.completed_time.wrapping_sub(start_data.start_time);
                if latency > 100000 {
                    
                }
                else {
                    // let latency_result = LatencyResult {
                //     request: start_data.request.clone(),
                //     latency,
                // };
                let r = start_data.clone().request;
                self.insert_latency_result(&k.0, &r, latency);
                // self.store_latency_result(k.0, latency_result);
                self.l_end_data_computing.remove(k);
                }
                
                
                
            }
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

    fn compute_scalability_throughput(&mut self, item: &TestItem) {
        self.prepare_compute_throughput();
        let current_time = Local::now().timestamp_millis() as u64;

        // let current_time = 30;

        let data_computing = self.t_end_data_computing.clone();
        data_computing.iter().for_each(|(k, _)| {
            if let Some(_) = self.t_start_data_computing.get(&k.1) {
                self.update_request_count_with_peer(k.0);

                self.t_end_data_computing.remove(k);
            }
        });
        
        let index = self.internal_round;
        self.internal_round += 1;
        let throughput_mid_results = self.throughput_mid_results.clone();
        throughput_mid_results
            .iter()
            .for_each(|(&peer_id, &count)| {
                println!(
                    "Item:{:?}; Count:{}; Round:{}; Internal:{}",
                    item, count, self.internal_round, self.throughput_internal
                );
                let throughput = (count as f64
                    / (0.001 * (self.internal_round * self.throughput_internal) as f64))
                    as u64;
                self.insert_scalability_throughput_result(item, index, &peer_id, throughput);
                // self.throughput_results.insert((index, peer_id), throughput);
            });

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

    fn compute_scalability_latency(&mut self, item: &TestItem) {
        self.prepare_compute_latency();

        let data_computing = self.l_end_data_computing.clone();
        data_computing.iter().for_each(|(k, end_data)| {
            if let Some(start_data) = self.l_start_data_computing.get(&k.1) {
                let latency = (end_data.completed_time - start_data.start_time) as u64;
                // let latency_result = LatencyResult {
                //     request: start_data.request.clone(),
                //     latency,
                // };
                let r = start_data.clone().request;
                println!("Start insert!");
                self.insert_scalability_latency_result(item, &k.0, &r, latency);
                println!("ENd insert!");
                // self.record_scalability_mid_latency(k.0, latency_result);
                self.l_end_data_computing.remove(k);
            };
        });

        // The remaining data is rewritten to l_start_data and l_end_data
        // self.l_start_data_computing.iter().for_each(|(k, v)| {
        //     self.l_start_data.insert(k.clone(), v.clone());
        // });
        self.l_end_data_computing.iter().for_each(|(k, v)| {
            self.l_end_data.insert(k.clone(), v.clone());
        });

        self.l_start_data_computing.clear();
        self.l_end_data_computing.clear();
    }

    pub fn compute_scalability(&mut self, node_count: u16, item: &TestItem) {
        self.compute_scalability_latency(item);
        self.compute_scalability_throughput(item);

        // let mut scalablity_results: HashMap<PeerId, ScalabilityResult> = HashMap::new();
        // let scalability_mid_throughputs = self.scalability_mid_throughputs.clone();
        // scalability_mid_throughputs
        //     .iter()
        //     .for_each(|(&peer_id, &throughput)| {
        //         if let Some(latency_results) = self.scalability_mid_latencies.get(&peer_id) {
        //             let scalabililty_result = ScalabilityResult {
        //                 throughput,
        //                 latency_results: latency_results.clone(),
        //             };
        //             scalablity_results.insert(peer_id, scalabililty_result);

        //             self.scalability_mid_throughputs.remove(&peer_id);
        //             self.scalability_mid_latencies.remove(&peer_id);
        //         } else {
        //             let scalabililty_result = ScalabilityResult {
        //                 throughput,
        //                 latency_results: vec![],
        //             };
        //             scalablity_results.insert(peer_id, scalabililty_result);

        //             self.scalability_mid_throughputs.remove(&peer_id);
        //         }
        //     });

        // self.scalability_mid_latencies
        //     .iter()
        //     .for_each(|(&peer_id, latency_results)| {
        //         let scalabililty_result = ScalabilityResult {
        //             throughput: 0,
        //             latency_results: latency_results.clone(),
        //         };
        //         scalablity_results.insert(peer_id, scalabililty_result);
        //     });

        // self.scalability_results
        //     .insert(node_count, scalablity_results);
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
                DataWarehouse::store_security_result(
                    &mut crash_results,
                    &k.0,
                    end_data.request.clone(),
                );
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

    pub fn test_malicious(&mut self, test_item: &TestItem) {
        let mut malicious_results = self.malicous_results(test_item);
        let data = self.s_end_data.clone();
        data.iter().for_each(|(k, end_data)| {
            DataWarehouse::store_security_result(
                &mut malicious_results,
                &k.0,
                end_data.request.clone(),
            );
            self.s_end_data.remove(k);
        });
        self.malicious_results
            .insert(test_item.clone(), malicious_results);
    }

    pub fn malicous_results(&mut self, test_item: &TestItem) -> HashMap<PeerId, SecurityResult> {
        if let Some(malicious_resuls) = self.malicious_results.get(test_item) {
            malicious_resuls.clone()
        } else {
            let malicious_results: HashMap<PeerId, SecurityResult> = HashMap::new();
            self.malicious_results
                .insert(test_item.clone(), malicious_results);
            self.malicious_results.get(test_item).unwrap().clone()
        }
    }

    pub fn store_security_result(
        security_results: &mut HashMap<PeerId, SecurityResult>,
        peer_id: &PeerId,
        request: Request,
    ) {
        if let Some(security_result_vec) = security_results.get_mut(&peer_id) {
            security_result_vec.0.push(request);
        } else {
            let r = SecurityResult(vec![request]);
            security_results.insert(*peer_id, r);
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
        self.internal_round = 0;
    }

    pub fn mysql_conn(&mut self) -> PooledConn {
        self.mysql_db.pool().get_conn().expect("Get Conn Error!")
    }

    pub fn create_result_tables(&mut self) {
        self.drop_tables();
        let mut conn = self.mysql_conn();

        conn.query_drop(
            r"CREATE TABLE throughput_results (
                id int auto_increment primary key,
                round int not null,
                peer_id text,
                throughput int not null
            )",
        )
        .expect("Create Table Error!");

        conn.query_drop(
            r"CREATE TABLE latency_results (
                id int auto_increment primary key,
                peer_id text not null,
                request_cmd text,
                latency int not null
            )",
        )
        .expect("Create Table Error!");

        conn.query_drop(
            r"CREATE TABLE scalability_items (
                id int auto_increment primary key,
                item text
            )",
        )
        .expect("Create Table Error!");

        conn.query_drop(
            r"CREATE TABLE scalability_throughput_results (
                id int auto_increment primary key,
                item text,
                round int not null,
                peer_id text,
                throughput int not null
            )",
        )
        .expect("Create Table Error!");

        conn.query_drop(
            r"CREATE TABLE scalability_latency_results (
                id int auto_increment primary key,
                item text,
                peer_id text not null,
                request_cmd text,
                latency int not null
            )",
        )
        .expect("Create Table Error!");
    }

    pub fn drop_tables(&mut self) {
        let mut conn = self.mysql_conn();

        conn.exec_drop("DROP TABLE throughput_results", ()).ok();
        conn.exec_drop("DROP TABLE latency_results", ()).ok();
        conn.exec_drop("DROP TABLE scalability_items", ()).ok();
        conn.exec_drop("DROP TABLE scalability_throughput_results", ())
            .ok();
        conn.exec_drop("DROP TABLE scalability_latency_results", ())
            .ok();
    }

    pub fn insert_throughput_result(&mut self, round: u64, peer_id: &PeerId, throughput: u64) {
        let mut conn = self.mysql_conn();

        conn.exec_drop(
            r"INSERT INTO throughput_results (round, peer_id, throughput) 
            VALUES (:round, :peer_id, :throughput)",
            params! {
                round,
                "peer_id" => peer_id.to_string(),
                throughput,
            },
        )
        .expect("Insert Throughput Result Error!");
    }

    pub fn query_throughput_results(&mut self, start: u64, count: u64) -> Vec<(u64, String, u64)> {
        let mut conn = self.mysql_conn();
        let sql = format!(
            "SELECT round, peer_id, throughput FROM throughput_results LIMIT {},{}",
            start, count
        );

        let results = conn
            .query_map(sql, |(round, peer_id, throughput)| {
                (round, peer_id, throughput)
            })
            .ok();
        if let None = results {
            return vec![];
        }

        results.unwrap()
    }

    pub fn insert_latency_result(&mut self, peer_id: &PeerId, request: &Request, latency: u64) {
        let mut conn = self.mysql_conn();

        conn.exec_drop(
            r"INSERT INTO latency_results (peer_id, request_cmd,  latency) 
            VALUES (:peer_id, :request_cmd,  :latency)",
            params! {
                "peer_id" => peer_id.to_string(),
                "request_cmd" => request.cmd.clone(),
                // "request_timestamp" => request.timestamp,
                latency,
            },
        )
        .expect("Insert Latency Result Error!");
    }

    pub fn query_latency_results(&mut self, start: u64, count: u64) -> Vec<(String, Request, u64)> {
        let mut conn = self.mysql_conn();
        let sql = format!("SELECT peer_id, request_cmd, latency FROM latency_results LIMIT {},{}", start, count);

        let results = conn
            .query_map(sql, |(peer_id, request_cmd, latency)| {
                let r = Request {
                    cmd: request_cmd,
                    // timestamp: request_timestamp,
                };
                (peer_id, r, latency)
            })
            .ok();
        if let None = results {
            return vec![];
        }

        results.unwrap()
    }

    pub fn insert_scalability_item(&mut self, item: &TestItem) {
        let mut conn = self.mysql_conn();

        conn.exec_drop(
            r"INSERT INTO scalability_items (item) 
            VALUES (:item)",
            params! {
                "item" => item.to_string(),
            },
        )
        .expect("Insert Scalability Item Error!");
    }

    pub fn query_scalability_items(&mut self) -> Vec<String> {
        let mut conn = self.mysql_conn();
        let sql = format!("SELECT item FROM scalability_items");

        let results = conn.query_map(sql, |item| item).ok();

        if let None = results {
            return vec![];
        }

        results.unwrap()
    }

    pub fn insert_scalability_throughput_result(
        &mut self,
        item: &TestItem,
        round: u64,
        peer_id: &PeerId,
        throughput: u64,
    ) {
        let mut conn = self.mysql_conn();

        conn.exec_drop(
            r"INSERT INTO scalability_throughput_results (item, round, peer_id, throughput) 
            VALUES (:item, :round, :peer_id, :throughput)",
            params! {
                "item" => item.to_string(),
                round,
                "peer_id" => peer_id.to_string(),
                throughput,
            },
        )
        .expect("Insert Scalability Throughput Result Error!");
    }

    pub fn query_scalability_throughput_results(
        &mut self,
        item: &str,
        start: u64,
        count: u64,
    ) -> Vec<(u64, String, u64)> {
        let mut conn = self.mysql_conn();
        let sql = format!(
            "SELECT round, peer_id, throughput FROM scalability_throughput_results WHERE item = '{}' LIMIT {},{}",
            item.to_string(), start, count
        );

        let results = conn
            .query_map(sql, |(round, peer_id, throughput)| {
                (round, peer_id, throughput)
            })
            .ok();
        if let None = results {
            return vec![];
        }

        results.unwrap()
    }

    pub fn insert_scalability_latency_result(
        &mut self,
        item: &TestItem,
        peer_id: &PeerId,
        request: &Request,
        latency: u64,
    ) {
        let mut conn = self.mysql_conn();

        conn.exec_drop(
            r"INSERT INTO scalability_latency_results (item, peer_id, request_cmd, latency) 
            VALUES (:item, :peer_id, :request_cmd, :latency)",
            params! {
                "item" => item.to_string(),
                "peer_id" => peer_id.to_string(),
                "request_cmd" => request.cmd.clone(),
                // "request_timestamp" => request.timestamp,
                latency,
            },
        )
        .expect("Insert Scalability Latency Result Error!");
    }

    pub fn query_scalability_latency_results(
        &mut self,
        item: &str,
        start: u64,
        count: u64,
    ) -> Vec<(String, Request, u64)> {
        let mut conn = self.mysql_conn();
        let sql = format!("SELECT peer_id, request_cmd, latency FROM scalability_latency_results WHERE item = '{}' LIMIT {},{}", item.to_string(), start, count);

        let results = conn
            .query_map(sql, |(peer_id, request_cmd, latency)| {
                let r = Request {
                    cmd: request_cmd,
                    // timestamp: request_timestamp,
                };
                (peer_id, r, latency)
            })
            .ok();
        if let None = results {
            return vec![];
        }

        results.unwrap()
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
            // timestamp: Local::now().timestamp_nanos() as u64,
        };
        let request_2 = Request {
            cmd: "request_2".to_string(),
            // timestamp: Local::now().timestamp_nanos() as u64,
        };
        let request_3 = Request {
            cmd: "request_3".to_string(),
            // timestamp: Local::now().timestamp_nanos() as u64,
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

        let mut data_warehouse =
            DataWarehouse::new("mysql://root:root@localhost:3306/bft_diagnosis");
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
