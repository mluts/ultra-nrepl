// use std::env::args;
use std::error;
use std::fmt;

use clap::clap_app;
// use clap::{App, Arg};

#[derive(Debug)]
enum OptsParseError {
    // PortNotSpecified,
    // OpNotSpecified,
    BadOpArg(String),
    BadUserInput(String),
}

impl fmt::Display for OptsParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OptsParseError: {}",
            match self {
                // OptsParseError::PortNotSpecified => "Port was not specified".to_string(),
                // OptsParseError::OpNotSpecified => "OP was not specified".to_string(),
                OptsParseError::BadOpArg(op_arg) => format!("Bad op arg: {}", op_arg),
                OptsParseError::BadUserInput(msg) => format!("Bad user input: {}", msg),
            }
        )
    }
}

impl error::Error for OptsParseError {}

#[derive(Debug)]
struct Opts {
    op: String,

    op_args: Vec<(String, String)>,

    port: u32,
}

fn parse_op_arg(s: &str) -> Option<(String, String)> {
    let parts = s.split("=").collect::<Vec<&str>>();

    match parts.len() {
        2 => Some((parts[0].to_string(), parts[1].to_string())),
        _ => None,
    }
}

impl Opts {
    fn parse() -> Result<Opts, OptsParseError> {
        let matches = clap_app!( ultra_op =>
            (version: "0.1")
            (author: "Michael Lutsiuk <michael.lutsiuk@gmail.com>")
            (about: "Sends OP to Nrepl and produces JSON output for response")
            (@arg OP: +required "Op to send")
            (@arg PORT: +takes_value +required -p --port "Nrepl port")
            (@arg OP_ARG: ... "Op Argument")
        )
        .get_matches();

        let op = matches.value_of("OP").unwrap();

        let port = matches
            .value_of("PORT")
            .unwrap()
            .parse::<u32>()
            .map_err(|e| OptsParseError::BadUserInput(format!("Failed to parse port: {}", e)))?;

        let op_args = matches
            .values_of("OP_ARG")
            .map(|vs| vs.map(|v| v.to_string()).collect::<Vec<String>>())
            .unwrap_or(vec![])
            .iter()
            .fold(
                Ok(vec![]),
                |acc: Result<Vec<(String, String)>, OptsParseError>, op_arg| {
                    acc.and_then(|mut op_opts| match parse_op_arg(op_arg) {
                        Some(args) => {
                            op_opts.push(args);
                            Ok(op_opts)
                        }
                        None => Err(OptsParseError::BadOpArg(op_arg.to_string())),
                    })
                },
            )?;

        let opts = Opts {
            op: op.to_string(),
            op_args: op_args,
            port: port,
        };

        Ok(opts)
    }
}

fn main() {
    let opts = Opts::parse();
    println!("{:?}", opts);
}
