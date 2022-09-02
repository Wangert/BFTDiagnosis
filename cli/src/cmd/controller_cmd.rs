use clap::Command;

// 启动控制器
pub fn init_cmd() -> Command<'static> {
    clap::Command::new("init")
        .about("init")
}

pub fn start_test_cmd() -> Command<'static> {
    clap::Command::new("test")
        .about("start_test")
        .subcommand(pbft_test_cmd())
        .subcommand(hotstuff_test_cmd())
        .subcommand(chain_hotstuff_test_cmd())
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