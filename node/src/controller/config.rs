use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct BFTDiagnosisConfig {
    pub throughput: Option<Throughput>,
    pub latency: Option<Latency>,
    pub scalability: Option<Scalability>,
    pub crash: Option<Crash>,
    pub malicious: Option<Malicious>,

}

#[derive(Debug, Deserialize, Clone)]
pub struct Throughput {
    pub enable: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Latency {
    pub enable: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Scalability {
    pub enable: Option<bool>,
    pub max: Option<u16>,
    pub internal: Option<u16>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Crash {
    pub enable: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Malicious {
    pub enable: Option<bool>,
    pub behaviour: Option<String>,
}