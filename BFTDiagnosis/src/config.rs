use std::{fs::File, io::Read};

use node::controller::config::BFTDiagnosisConfig;
use serde::Deserialize;

// #[derive(Debug, Deserialize, Clone)]
// pub struct BFTDiagnosisConfig {
//     pub throughput: Option<Throughput>,
//     pub latency: Option<Latency>,
//     pub scalability: Option<Scalability>,
//     pub crash: Option<Crash>,
//     pub malicious: Option<Malicious>,

// }

// #[derive(Debug, Deserialize, Clone)]
// pub struct Throughput {
//     pub enable: Option<bool>,
// }

// #[derive(Debug, Deserialize, Clone)]
// pub struct Latency {
//     pub enable: Option<bool>,
// }

// #[derive(Debug, Deserialize, Clone)]
// pub struct Scalability {
//     pub enable: Option<bool>,
//     pub max: Option<u16>,
//     pub internal: Option<u16>,
// }

// #[derive(Debug, Deserialize, Clone)]
// pub struct Crash {
//     pub enable: Option<bool>,
// }

// #[derive(Debug, Deserialize, Clone)]
// pub struct Malicious {
//     pub enable: Option<bool>,
// }

#[derive(Debug, Deserialize, Clone)]
pub struct ControllerConfig {
    pub network: Option<Network>,
    pub extra: Option<Extra>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnalyzerConfig {
    pub network: Option<Network>,
    pub execution: Option<Execution>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Network {
    pub ip_addr: Option<String>,
    pub ip_port: Option<u16>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Execution {
    pub performance_duration: Option<u64>,
    pub performance_internal: Option<u64>,
    pub crash_duration: Option<u64>,
    pub malicious_duration: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Extra {
    pub conspire_request_send_duration: Option<u64>,
}

pub fn read_controller_config() -> ControllerConfig {
    let config_file_path = "./BFTDiagnosis/src/config_files/controller_config.toml";
    let mut config_file = match File::open(config_file_path) {
        Ok(file) => file,
        Err(e) => panic!("Read controller config file error: {}", e)
    };

    let mut str_val = String::new();
    match config_file.read_to_string(&mut str_val) {
        Ok(s) => s,
        Err(e) => panic!("Read file error: {}", e)
    };

    let controller_conifg: ControllerConfig = toml::from_str(&str_val).unwrap();

    controller_conifg
}

pub fn read_analyzer_config() -> AnalyzerConfig {
    let config_file_path = "./BFTDiagnosis/src/config_files/analyzer_config.toml";
    let mut config_file = match File::open(config_file_path) {
        Ok(file) => file,
        Err(e) => panic!("Read analyzer config file error: {}", e)
    };

    let mut str_val = String::new();
    match config_file.read_to_string(&mut str_val) {
        Ok(s) => s,
        Err(e) => panic!("Read file error: {}", e)
    };

    let analyzer_conifg: AnalyzerConfig = toml::from_str(&str_val).unwrap();

    analyzer_conifg
}

pub fn read_bft_diagnosis_config() -> BFTDiagnosisConfig {
    let config_file_path = "./BFTDiagnosis/src/config_files/bft_diagnosis_config.toml";
    let mut config_file = match File::open(config_file_path) {
        Ok(file) => file,
        Err(e) => panic!("Read bft_diagnosis config file error: {}", e)
    };

    let mut str_val = String::new();
    match config_file.read_to_string(&mut str_val) {
        Ok(s) => s,
        Err(e) => panic!("Read file error: {}", e)
    };

    let bft_diagnosis_conifg: BFTDiagnosisConfig = toml::from_str(&str_val).unwrap();

    // println!("{:#?}", &bft_diagnosis_conifg);
    bft_diagnosis_conifg

    
}

#[cfg(test)]
pub mod config_test {
    use crate::config::read_controller_config;

    #[test]
    pub fn read_controller_config_works() {
        let controller_config = read_controller_config();

        println!("{:#?}", controller_config);
    }
}