use clap::{clap_app, ArgMatches};
use ultra_nrepl::cmd;
use ultra_nrepl::nrepl;

// fn show_session_id(nrepl_stream: &nrepl::NreplStream) {
//     let sid = ultra_nrepl::nrepl::session::get_existing_session_id(&nrepl_stream).unwrap();
//     println!("Session id: {}", sid);
// }

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
        match nrepl::NreplStream::connect_timeout(&nrepl::port_addr(port)) {
            Ok(nrepl) => nrepl,
            Err(e) => cmd::die_err(&format!("Failed to connect to nrepl: {}", e)),
        }
    } else {
        cmd::die_err("Please specify nrepl PORT")
    }
}

fn main() {
    let mut app = clap_app!(ultra_nrepl =>
        (version: "0.1")
        (author: "Michael Lutsiuk <michael.lutsiuk@gmail.com>")
        (@arg PORT: +takes_value -p --port "Nrepl port")
    )
    .subcommand(cmd::op::app())
    .subcommand(cmd::find_def::app())
    .subcommand(cmd::doc::app());

    let matches = app.clone().get_matches();
    let nrepl_stream = nrepl_stream(&matches);

    match matches.subcommand() {
        ("op", Some(argm)) => cmd::op::run(&argm, &nrepl_stream),
        ("find_def", Some(argm)) => cmd::find_def::run(&argm, &nrepl_stream),
        ("doc", Some(argm)) => cmd::doc::run(&argm, &nrepl_stream),
        // ("session_id", Some(_)) => show_session_id(&nrepl_stream),
        _ => {
            app.print_help().unwrap();
            println!("\n")
        }
    }
}
