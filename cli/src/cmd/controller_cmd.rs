use clap::Command;

// 启动控制器
pub fn init_cmd() -> Command<'static> {
    clap::Command::new("init")
        .about("init")
}

pub fn print_unfinished_test_items_cmd() -> Command<'static> {
    clap::Command::new("printUnfinishedTestItems")
        .about("printUnfinishedTestItems")
}

pub fn print_throughput_results_cmd() -> Command<'static> {
    clap::Command::new("printThroughputResults")
        .about("printThroughputResults")
}

pub fn print_latency_results_cmd() -> Command<'static> {
    clap::Command::new("printLatencyResults")
        .about("printLatencyResults")
}

pub fn print_scalability_results_cmd() -> Command<'static> {
    clap::Command::new("printScalabilityResults")
        .about("printScalabilityResults")
}

pub fn start_test_cmd() -> Command<'static> {
    clap::Command::new("startTest")
        .about("startTest")
}

pub fn configure_analyzer_cmd() -> Command<'static> {
    clap::Command::new("configureAnalyzer")
        .about("configureAnalyzer")
}

pub fn configure_consensus_node_cmd() -> Command<'static> {
    clap::Command::new("configureConsensusNode")
        .about("configureConsensusNode")
}

pub fn protocol_start_cmd() -> Command<'static> {
    clap::Command::new("protocolStart")
        .about("protocolStart")
}

pub fn pbft_test_cmd() -> Command<'static> {
    clap::Command::new("pbft").about("PBFT")
}

pub fn hotstuff_test_cmd() -> Command<'static> {
    clap::Command::new("hotstuff").about("Hotstuff")
}

pub fn chain_hotstuff_test_cmd() -> Command<'static> {
    clap::Command::new("chain_hotstuff").about("Chain-Hotstuff")
}