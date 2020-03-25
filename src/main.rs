use clap::{clap_app, ArgMatches};
use unrepl::cmd;
use unrepl::nrepl;
use unrepl::nrepl::ops;
use unrepl::nrepl::NreplOp;

fn nrepl_stream(arg: &ArgMatches) -> nrepl::NreplStream {
    let port = if let Some(port_str) = arg.value_of("PORT") {
        match port_str.parse::<u32>() {
            Ok(port) => Some(port),
            _ => cmd::die_err(&format!("Bad port value: {}", port_str)),
        }
    } else {
        nrepl::default_nrepl_port()
    };

    if let Some(port) = port {
        match nrepl::NreplStream::new(&nrepl::port_addr(port)) {
            Ok(nrepl) => nrepl,
            Err(e) => cmd::die_err(&format!("Failed to connect to nrepl: {}", e)),
        }
    } else {
        cmd::die_err("Please specify nrepl PORT")
    }
}

fn show_ns(argm: &ArgMatches, n: &nrepl::NreplStream) {
    let file = argm.value_of("FILE").unwrap();
    let session = cmd::die_if_err(unrepl::nrepl::session::get_existing_session_id(n));
    let op = ops::GetNsName::new(file.to_string(), session);
    println!("NS: {}", op.send(n).unwrap().unwrap());
}

fn main() {
    unrepl::config::ensure_config_dir().unwrap();
    unrepl::config::ensure_migrations().unwrap();

    let mut app = clap_app!(unrepl =>
        (version: "0.1")
        (author: "Michael Lutsiuk <michael.lutsiuk@gmail.com>")
        (@arg PORT: +takes_value -p --port "Nrepl port")
    )
    .subcommand(clap_app!(show_ns => (@arg FILE: +takes_value "File")))
    .subcommand(cmd::op::app())
    .subcommand(cmd::find_def::app())
    .subcommand(cmd::read_jar::app())
    .subcommand(cmd::doc::app());

    let matches = app.clone().get_matches();
    let nrepl_stream = nrepl_stream(&matches);

    match matches.subcommand() {
        ("op", Some(argm)) => cmd::op::run(&argm, &nrepl_stream),
        ("find_def", Some(argm)) => cmd::find_def::run(&argm, &nrepl_stream),
        ("doc", Some(argm)) => cmd::doc::run(&argm, &nrepl_stream),
        ("show_ns", Some(argm)) => show_ns(&argm, &nrepl_stream),
        ("read_jar", Some(argm)) => cmd::read_jar::run(&argm),
        _ => {
            app.print_help().unwrap();
            println!("\n")
        }
    }
}
