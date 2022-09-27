use crate::cmd::controller_cmd::configure_analyzer_cmd;
use crate::cmd::controller_cmd::configure_consensus_node_cmd;
use crate::cmd::controller_cmd::init_cmd;
use crate::cmd::controller_cmd::print_latency_results_cmd;
use crate::cmd::controller_cmd::print_protocol_phases_cmd;
use crate::cmd::controller_cmd::print_scalability_results_cmd;
use crate::cmd::controller_cmd::print_throughput_results_cmd;
use crate::cmd::controller_cmd::print_unfinished_test_items_cmd;
use crate::cmd::controller_cmd::protocol_start_cmd;
use crate::cmd::controller_cmd::query_protocol_phases_cmd;
use crate::cmd::controller_cmd::start_test_cmd;
use crate::commons::CommandCompleter;
use crate::commons::SubCmd;

use clap::Arg;
use clap::Command as clap_Command;
use lazy_static::lazy_static;

use std::borrow::Borrow;

use sysinfo::{PidExt, System, SystemExt};

lazy_static! {
    pub static ref CMD: clap::Command<'static> = clap::Command::new("BFTDiagnosis")
        .version("1.0")
        .author("Daslab")
        .about("BFTDiagnosis")
        .arg(
            Arg::new("consensus")
                .short('n')
                .long("consensus")
                .help("-n")
                .takes_value(true)
                .multiple_values(true)
        )
        .arg(
            Arg::new("controller")
                .short('c')
                .long("controller")
                .help("-c")
        )
        .arg(Arg::new("analyzer").short('a').long("analyzer").help("-a"))
        .help_expected(true)
        .subcommand(init_cmd())
        .subcommand(print_unfinished_test_items_cmd())
        .subcommand(print_throughput_results_cmd())
        .subcommand(print_latency_results_cmd())
        .subcommand(print_scalability_results_cmd())
        .subcommand(start_test_cmd())
        .subcommand(configure_analyzer_cmd())
        .subcommand(protocol_start_cmd())
        .subcommand(configure_consensus_node_cmd())
        .subcommand(query_protocol_phases_cmd())
        .subcommand(print_protocol_phases_cmd());
    static ref CMD_SUBCMDS: Vec<SubCmd> = subcommands();
}

lazy_static! {
    pub static ref CONTROLLER_CMD: clap::Command<'static> = clap::Command::new("BFTDiagnosis")
        .version("1.0")
        .author("Daslab")
        .about("BFTDiagnosis")
        .arg(
            Arg::new("controller")
                .short('c')
                .long("controller")
                .help("-c")
        )
        .help_expected(true)
        .subcommand(init_cmd())
        .subcommand(start_test_cmd());
    static ref CONTROLLER_SUBCMDS: Vec<SubCmd> = subcommands();
}

lazy_static! {
    pub static ref ANALYZER_CMD: clap::Command<'static> = clap::Command::new("BFTDiagnosis")
        .version("1.0")
        .author("Daslab")
        .about("BFTDiagnosis")
        .help_expected(true)
        .subcommand(init_cmd())
        .subcommand(start_test_cmd());
    static ref ANALYZER_SUBCMDS: Vec<SubCmd> = subcommands();
}

// 获取全部子命令，用于构建commandcompleter
pub fn all_subcommand(app: &clap_Command, beginlevel: usize, input: &mut Vec<SubCmd>) {
    let nextlevel = beginlevel + 1;
    let mut subcmds = vec![];
    for iterm in app.get_subcommands() {
        subcmds.push(iterm.get_name().to_string());
        if iterm.has_subcommands() {
            all_subcommand(iterm, nextlevel, input);
        } else {
            if beginlevel == 0 {
                all_subcommand(iterm, nextlevel, input);
            }
        }
    }
    let subcommand = SubCmd {
        level: beginlevel,
        command_name: app.get_name().to_string(),
        subcommands: subcmds,
    };
    input.push(subcommand);
}

pub fn get_command_completer() -> CommandCompleter {
    CommandCompleter::new(CMD_SUBCMDS.to_vec())
}

fn subcommands() -> Vec<SubCmd> {
    let mut subcmds = vec![];
    all_subcommand(CMD.clone().borrow(), 0, &mut subcmds);
    subcmds
}

pub fn process_exists(pid: &u32) -> bool {
    let mut sys = System::new_all();
    sys.refresh_all();
    for (syspid, _) in sys.processes() {
        if syspid.as_u32().eq(pid) {
            return true;
        }
    }
    return false;
}
