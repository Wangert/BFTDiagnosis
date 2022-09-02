use std::{fs::File, io::Read};

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ControllerConfig {
    pub network: Option<Network>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnalyzerConfig {
    pub network: Option<Network>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Network {
    pub ip_addr: Option<String>,
    pub ip_port: Option<u16>,
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

#[cfg(test)]
pub mod config_test {
    use crate::config::read_controller_config;

    #[test]
    pub fn read_controller_config_works() {
        let controller_config = read_controller_config();

        println!("{:#?}", controller_config);
    }
}