use crate::cmd;
use crate::jar;
use clap::{clap_app, App, ArgMatches};

pub fn app<'a, 'b>() -> App<'a, 'b> {
    clap_app!(read_jar =>
            (about: "Shows jar's single file content")
            (@arg JAR: +takes_value +required "Path to JAR file")
            (@arg FILE: +takes_value +required "Path to file inside JAR")
    )
}

pub fn run(matches: &ArgMatches) {
    let jar = matches.value_of("JAR").unwrap().to_string();
    let file = matches.value_of("FILE").unwrap().to_string();

    let contents = cmd::die_if_err(jar::read_jar_file(jar, file));

    println!("{}", contents);
}
