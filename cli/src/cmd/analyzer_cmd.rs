use clap::Command;

// 启动分析器
pub fn init_cmd() -> Command<'static> {
    clap::Command::new("init")
        .about("init")
        // .subcommand(get_baidu_cmd())
}

