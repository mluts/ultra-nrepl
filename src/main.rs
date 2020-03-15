use clap::clap_app;
use ultra_nrepl::cmd::op;

fn main() {
    let mut app = clap_app!(ultra_nrepl =>
        (version: "0.1")
        (author: "Michael Lutsiuk <michael.lutsiuk@gmail.com>")
    )
    .subcommand(op::app());

    let matches = app.clone().get_matches();

    match matches.subcommand() {
        ("op", Some(argm)) => op::run(&argm),
        _ => app.print_help().unwrap(),
    }
}
