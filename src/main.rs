use clap::{clap_app, App, ArgMatches};
use ultra_nrepl::cmd::op;

fn show_session_id_app<'a, 'b>() -> App<'a, 'b> {
    clap_app!(session_id =>
        (about: "Shows current session-id")
        (@arg PORT: +takes_value +required -p --port "Nrepl Port")
    )
}

fn show_session_id(argm: &ArgMatches) {
    let port = argm.value_of("PORT").unwrap().parse::<u32>().unwrap();
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let nrepl_stream = ultra_nrepl::nrepl::NreplStream::connect_timeout(&addr).unwrap();
    let sid = ultra_nrepl::nrepl::session::get_existing_session_id(&nrepl_stream).unwrap();
    println!("Session id: {}", sid);
}

fn main() {
    let mut app = clap_app!(ultra_nrepl =>
        (version: "0.1")
        (author: "Michael Lutsiuk <michael.lutsiuk@gmail.com>")
    )
    .subcommand(op::app())
    .subcommand(show_session_id_app());

    let matches = app.clone().get_matches();

    match matches.subcommand() {
        ("op", Some(argm)) => op::run(&argm),
        ("session_id", Some(argm)) => show_session_id(argm),
        _ => {
            app.print_help().unwrap();
            println!("\n")
        }
    }
}
