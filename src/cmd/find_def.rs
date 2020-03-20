use crate::cmd;
use crate::nrepl;
use crate::nrepl::ops;
use crate::nrepl::session;
use crate::nrepl::NreplOp;
use clap::{clap_app, App, ArgMatches};
use failure::Fail;
use std::path::Path;

struct Opts {
    file: String,
    symbol: String,
}

enum File {
    Jar { jar: String, file: String },
    File(String),
}

#[derive(Debug, Fail)]
enum FileError {
    #[fail(display = "File format returned from Nrepl is not correct: {}", _0)]
    IncorrectPathFormat(String),
}

impl Opts {
    fn parse(matches: &ArgMatches) -> Opts {
        let file = matches.value_of("FILE").unwrap().to_string();
        let symbol = matches.value_of("SYMBOL").unwrap().to_string();

        let file = Path::new(&file)
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        Opts { file, symbol }
    }
}

pub fn app<'a, 'b>() -> App<'a, 'b> {
    clap_app!(find_def =>
        (about: "Shows position of ns/symbol")
        (@arg FILE: +required "FILE with NS containing symbol")
        (@arg SYMBOL: +required "SYMBOL")
    )
}

fn parse_file(path: String) -> Result<File, FileError> {
    let parts: Vec<&str> = path.split(":").collect();

    let first_part = parts.get(0).unwrap();

    match *first_part {
        "jar" => {
            let jar_path = parts.get(2).unwrap();
            let jar_parts: Vec<&str> = jar_path.split("!").collect();

            Ok(File::Jar {
                jar: jar_parts.get(0).unwrap().to_string(),
                file: jar_parts.get(1).unwrap().to_string(),
            })
        }
        "file" => Ok(File::File(parts.get(1).unwrap().to_string())),
        _ => Err(FileError::IncorrectPathFormat(path)),
    }
}

pub fn run(matches: &ArgMatches, nrepl_stream: &nrepl::NreplStream) {
    let opts = Opts::parse(matches);
    let session = cmd::die_if_err(session::get_existing_session_id(nrepl_stream));
    let ns = cmd::die_if_err(ops::GetNsName::new(opts.file).send(nrepl_stream));

    if ns.is_none() {
        cmd::die_err("File doesn't have NS declaration");
    }

    let op = ops::Info::new(session, ns.unwrap(), opts.symbol);
    let res = cmd::die_if_err(op.send(nrepl_stream));

    if let Some(res) = res {
        match res {
            ops::InfoResponseType::Ns(res) => {
                let f = parse_file(res.file).unwrap();

                let line = res.line.to_string();

                let mut data = vec![
                    ("IS-NS", "TRUE".to_string()),
                    ("LINE", line.to_string()),
                    ("COLUMN", "1".to_string()),
                    ("RESOURCE", res.resource),
                ];

                match f {
                    File::Jar { jar, file } => {
                        data.push(("JAR", jar));
                        data.push(("FILE", file))
                    }

                    File::File(file) => data.push(("FILE", file)),
                }

                cmd::print_parseable(&data);
            }

            ops::InfoResponseType::Symbol(res) => {
                let f = parse_file(res.file).unwrap();

                let mut data = vec![
                    ("IS-SYMBOL", "TRUE".to_string()),
                    ("LINE", res.line.to_string()),
                    ("COLUMN", res.col.unwrap().to_string()),
                    ("RESOURCE", res.resource),
                ];

                match f {
                    File::Jar { jar, file } => {
                        data.push(("JAR", jar));
                        data.push(("FILE", file))
                    }

                    File::File(file) => data.push(("FILE", file)),
                }

                cmd::print_parseable(&data);
            }
        }
    } else {
        cmd::print_parseable(&vec![("IS-EMPTY", "TRUE".to_string())]);
    }
}
