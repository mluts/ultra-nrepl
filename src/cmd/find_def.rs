use clap::{clap_app, App, ArgMatches};
use crate::nrepl;

struct Opts {
    ns: String,
    symbol: String,
}

impl Opts {
    fn parse(matches: &ArgMatches) -> Opts {
        let ns = matches.value_of("NS").unwrap().to_string();
        let symbol = matches.value_of("SYMBOL").unwrap().to_string();

        Opts { ns, symbol }
    }
}

pub fn app<'a, 'b>() -> App<'a, 'b> {
    clap_app!(find_def =>
        (about: "Shows FILE, COLUMN and POSITION for given ns/symbol")
        (@arg NS: +required "NS")
        (@arg SYMBOL: +required "SYMBOL")
    )
}

pub fn run(matches: &ArgMatches, nrepl_stream: &nrepl::NreplStream) {
    let opts = Opts::parse(matches);
}
