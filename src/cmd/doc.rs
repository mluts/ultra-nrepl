use crate::cmd;
use crate::nrepl;
use crate::nrepl::ops;
use crate::nrepl::session;
use crate::nrepl::NreplOp;
use clap::{clap_app, App, ArgMatches};

struct Opts {
    file: String,
    symbol: String,
}

impl Opts {
    fn parse(matches: &ArgMatches) -> Opts {
        let file = matches.value_of("FILE").unwrap().to_string();
        let symbol = matches.value_of("SYMBOL").unwrap().to_string();

        Opts { file, symbol }
    }
}

pub fn app<'a, 'b>() -> App<'a, 'b> {
    clap_app!(doc =>
        (about: "Shows DOC for symbol")
        (@arg FILE: +required "FILE with NS containing SYMBOL")
        (@arg SYMBOL: +required "SYMBOL")
    )
}

pub fn run(matches: &ArgMatches, nrepl_stream: &nrepl::NreplStream) {
    let opts = Opts::parse(matches);
    let session = cmd::die_if_err(session::get_existing_session_id(nrepl_stream));
    let ns = cmd::die_if_err(ops::GetNsName::new(opts.file, session.clone()).send(nrepl_stream));
    let op = ops::Info::new(session, ns.unwrap(), opts.symbol);
    let res = cmd::die_if_err(op.send(nrepl_stream));

    if let Some(res) = res {
        println!("{}", res.into_resp().doc);
    }
}
