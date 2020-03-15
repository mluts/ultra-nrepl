use std::fmt;
use std::error;
use clap::{App, ArgMatches, clap_app};
use crate::nrepl;

#[derive(Debug)]
enum OpOptsParseError {
    BadOpArg(String),
    BadUserInput(String),
}

impl fmt::Display for OpOptsParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OpOptsParseError: {}",
            match self {
                OpOptsParseError::BadOpArg(op_arg) => format!("Bad op arg: {}", op_arg),
                OpOptsParseError::BadUserInput(msg) => format!("Bad user input: {}", msg),
            }
        )
    }
}

impl error::Error for OpOptsParseError {}

#[derive(Debug)]
struct OpOpts {
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

impl OpOpts {
    fn parse(matches: &ArgMatches) -> Result<OpOpts, OpOptsParseError> {
        let op = matches.value_of("OP").unwrap();

        let port = matches
            .value_of("PORT")
            .unwrap()
            .parse::<u32>()
            .map_err(|e| OpOptsParseError::BadUserInput(format!("Failed to parse port: {}", e)))?;

        let op_args = matches
            .values_of("OP_ARG")
            .map(|vs| vs.map(|v| v.to_string()).collect::<Vec<String>>())
            .unwrap_or(vec![])
            .iter()
            .fold(
                Ok(vec![]),
                |acc: Result<Vec<(String, String)>, OpOptsParseError>, op_arg| {
                    acc.and_then(|mut op_opts| match parse_op_arg(op_arg) {
                        Some(args) => {
                            op_opts.push(args);
                            Ok(op_opts)
                        }
                        None => Err(OpOptsParseError::BadOpArg(op_arg.to_string())),
                    })
                },
            )?;

        let opts = OpOpts {
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
    match OpOpts::parse(matches) {
        Ok(opts) => {
            let addr: std::net::SocketAddr = format!("127.0.0.1:{}", opts.port).parse().unwrap();

            let nrepl_stream = nrepl::NreplStream::connect_timeout(&addr).unwrap();
            let op = nrepl::Op::new(opts.op, opts.op_args);

            for resp in nrepl_stream.op(&op).unwrap() {
                println!("{}", nrepl::to_json_string(&resp).unwrap());
            }
        }
        Err(e) => eprintln!("Parse error: {}", e),
    }
}
