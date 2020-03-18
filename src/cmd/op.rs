use crate::nrepl;
use clap::{clap_app, App, ArgMatches};
use serde_json::error as json_error;
use serde_json::value::Value as JsonValue;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
enum OptsParseError {
    BadOpArg(String),
}

impl fmt::Display for OptsParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OptsParseError: {}",
            match self {
                OptsParseError::BadOpArg(op_arg) => format!("Bad op arg: {}", op_arg),
            }
        )
    }
}

#[derive(Debug)]
struct Opts {
    op: String,

    op_args: Vec<(String, String)>,
}

pub fn to_json_string(resp: &nrepl::Resp) -> Result<String, json_error::Error> {
    let mut hm: HashMap<String, JsonValue> = HashMap::new();

    for (k, v) in resp.iter() {
        hm.insert(
            k.to_string(),
            crate::bencode::to_json_value(v.clone()).unwrap(),
        );
    }

    serde_json::to_string(&hm)
}

fn parse_op_arg(s: &str) -> Option<(String, String)> {
    let parts = s.split("=").collect::<Vec<&str>>();

    match parts.len() {
        2 => Some((parts[0].to_string(), parts[1].to_string())),
        _ => None,
    }
}

impl Opts {
    fn parse(matches: &ArgMatches) -> Result<Opts, OptsParseError> {
        let op = matches.value_of("OP").unwrap();

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
            op_args,
        };

        Ok(opts)
    }
}

pub fn app<'a, 'b>() -> App<'a, 'b> {
    clap_app!(op =>
        (about: "Sends OP to Nrepl and produces JSON output for response")
        (@arg OP: +required "Op to send")
        (@arg OP_ARG: ... "Op Argument")
    )
}

pub fn run(matches: &ArgMatches, nrepl_stream: &nrepl::NreplStream) {
    match Opts::parse(matches) {
        Ok(opts) => {
            let op = nrepl::Op::new(opts.op, opts.op_args);

            for resp in nrepl_stream.op(op).unwrap().into_resps() {
                println!("{}", to_json_string(&resp).unwrap());
            }
        }
        Err(e) => eprintln!("Parse error: {}", e),
    }
}
