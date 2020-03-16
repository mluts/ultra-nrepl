use crate::nrepl;
use clap::{clap_app, App, ArgMatches};
use serde_bencode::value::Value as BencodeValue;
use serde_json::error as json_error;
use serde_json::value::Value as JsonValue;
use std::collections::HashMap;
use std::error;
use std::fmt;

#[derive(Debug)]
enum OptsParseError {
    BadOpArg(String),
    BadUserInput(String),
}

impl fmt::Display for OptsParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OptsParseError: {}",
            match self {
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

pub fn to_json_string(resp: &nrepl::Resp) -> Result<String, json_error::Error> {
    let mut hm: HashMap<String, JsonValue> = HashMap::new();

    for (k, v) in resp.iter() {
        hm.insert(
            k.to_string(),
            match v {
                BencodeValue::Bytes(s) => JsonValue::String(String::from_utf8(s.to_vec()).unwrap()),
                BencodeValue::List(ls) => JsonValue::Array(
                    ls.into_iter()
                        .map(|e| {
                            JsonValue::String(if let BencodeValue::Bytes(s) = e {
                                String::from_utf8(s.to_vec()).unwrap()
                            } else {
                                "BENCODE".to_string()
                            })
                        })
                        .collect(),
                ),

                _ => JsonValue::String("BENCODE VALUE".to_string()),
            },
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

pub fn app<'a, 'b>() -> App<'a, 'b> {
    clap_app!(op =>
        (about: "Sends OP to Nrepl and produces JSON output for response")
        (@arg OP: +required "Op to send")
        (@arg PORT: +takes_value +required -p --port "Nrepl port")
        (@arg OP_ARG: ... "Op Argument")
    )
}

pub fn run(matches: &ArgMatches) {
    match Opts::parse(matches) {
        Ok(opts) => {
            let addr: std::net::SocketAddr = format!("127.0.0.1:{}", opts.port).parse().unwrap();

            let nrepl_stream = nrepl::NreplStream::connect_timeout(&addr).unwrap();
            let op = nrepl::Op::new(opts.op, opts.op_args);

            for resp in nrepl_stream.op(op).unwrap() {
                println!("{}", to_json_string(&resp).unwrap());
            }
        }
        Err(e) => eprintln!("Parse error: {}", e),
    }
}
