use clap::{clap_app, ArgMatches};
use ultra_nrepl::cmd::op;
use ultra_nrepl::nrepl;

fn show_session_id(nrepl_stream: &nrepl::NreplStream) {
    let sid = ultra_nrepl::nrepl::session::get_existing_session_id(&nrepl_stream).unwrap();
    println!("Session id: {}", sid);
}

fn die_err(msg: &str) -> ! {
    eprintln!("ERROR: {}", msg);
    std::process::exit(1);
}

fn nrepl_stream(arg: &ArgMatches) -> nrepl::NreplStream {
    let port = if let Some(port_str) = arg.value_of("PORT") {
        match port_str.parse::<u32>() {
            Ok(port) => Some(port),
            _ => die_err(&format!("Bad port value: {}", port_str)),
        }
    } else {
        nrepl::default_nrepl_port()
    };

    if let Some(port) = port {
        match nrepl::NreplStream::connect_timeout(&nrepl::port_addr(port)) {
            Ok(nrepl) => nrepl,
            Err(e) => die_err(&format!("Failed to connect to nrepl: {}", e)),
        }
    } else {
        die_err("Please specify nrepl PORT")
    }
}

fn main() {
    let mut app = clap_app!(ultra_nrepl =>
        (version: "0.1")
        (author: "Michael Lutsiuk <michael.lutsiuk@gmail.com>")
        (@arg PORT: +takes_value -p --port "Nrepl port")
    )
    .subcommand(op::app());

    let matches = app.clone().get_matches();
    let nrepl_stream = nrepl_stream(&matches);

    match matches.subcommand() {
        ("op", Some(argm)) => op::run(&argm, &nrepl_stream),
        ("session_id", Some(_)) => show_session_id(&nrepl_stream),
        _ => {
            app.print_help().unwrap();
            println!("\n")
        }
    }
}
