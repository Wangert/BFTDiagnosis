use clap::{ArgMatches, Command};
use tokio::sync::mpsc::Sender;

use crate::cmd::rootcmd::CONTROLLER_CMD;
use crate::cmd::{get_command_completer};
use crate::commons::CommandCompleter;
use log::error;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{MatchingBracketValidator, Validator};
use rustyline::{validate, CompletionType, Config, Context, Editor, OutputStreamType};
use rustyline_derive::Helper;
use shellwords::split;
use std::borrow::Cow::{self, Borrowed, Owned};
use std::process::{exit};

use crate::interact;

#[derive(Debug, Clone)]
pub enum ClientType {
    Controller,
    Analyzer,
}

#[derive(Debug, Clone)]
pub struct Client {
    arg_matches: ArgMatches,
}

impl Client {
    pub fn new(arg_matches: ArgMatches) -> Self {
        Self { arg_matches }
    }

    pub fn arg_matches(&mut self) -> ArgMatches {
        self.arg_matches.clone()
    }

    pub fn run(&mut self, args_sender: Sender<Vec<String>>, client_type: ClientType) {
        tokio::spawn(async move {
            run(args_sender, client_type).await;
        });
    }
}

pub async fn run(args_sender: Sender<Vec<String>>, client_type: ClientType) {
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .output_stream(OutputStreamType::Stdout)
        .build();

    let h = MyHelper {
        completer: get_command_completer(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter {},
        colored_prompt: "".to_owned(),
        validator: MatchingBracketValidator::new(),
    };

    let mut rl = Editor::with_config(config);
    rl.set_helper(Some(h));

    // if rl.load_history("/tmp/history").is_err() {

        println!("============================================================================================================");
        println!("************************************************************************************************************");
        println!("                                                                                                            ");
        println!("
        
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
        
        ");
        println!("BFTDiagnosis Tool(version 1.0):");
        println!("<<A general testing framework for semi-synchronous BFT protocol>>");
        println!("                                                                                                           ");
        println!("                                                                     Developers: Jitao Wang & Bo Zhang");
        println!("                                                                     Laboratory: DasLab of Fudan University");
        println!("                                                                                                           ");
        println!("***********************************************************************************************************");
        println!("===========================================================================================================");
    // }

    loop {
        let mut p = "BFTDiagnosis(Controller)>> ".to_string();
        match client_type {
            ClientType::Analyzer => {
                p = "BFTDiagnosis(Analyzer)>>".to_string()
            }
            _ => {}
        }
        // let p = format!("{}>> ", "BFTDiagnosis");
        rl.helper_mut().expect("No helper").colored_prompt = format!("\x1b[1;32m{}\x1b[0m", p);
        let readline = rl.readline(&p);
        match readline {
            Ok(line) => {
                if line.trim_start().is_empty() {
                    continue;
                }

                rl.add_history_entry(line.as_str());
                match split(line.as_str()).as_mut() {
                    Ok(arg) => {
                        if arg[0] == "exit" {
                            println!("bye!");
                            break;
                        }
                        arg.insert(0, "clisample".to_string());
                        _ = args_sender.send(arg.to_vec()).await;
                    }
                    Err(err) => {
                        println!("{}", err)
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    rl.append_history("/tmp/history")
        .map_err(|err| error!("{}", err))
        .ok();
}

#[derive(Helper)]
pub struct MyHelper {
    // completer: FileCompleter,
    completer: CommandCompleter,
    highlighter: MatchingBracketHighlighter,
    validator: MatchingBracketValidator,
    hinter: HistoryHinter,
    colored_prompt: String,
}

impl Completer for MyHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for MyHelper {
    type Hint = String;
    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx);
        Some("".to_string())
    }
}

impl Highlighter for MyHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
}

impl Validator for MyHelper {
    fn validate(
        &self,
        ctx: &mut validate::ValidationContext,
    ) -> rustyline::Result<validate::ValidationResult> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}
