pub mod find_def;
pub mod op;
pub mod doc;
pub mod read_jar;

pub fn die_err(msg: &str) -> ! {
    eprintln!("{}", msg);
    std::process::exit(1);
}

pub fn print_parseable(data: &Vec<(&str, String)>) {
    for (k, v) in data {
        println!("{} {}", k.to_uppercase(), v)
    }
}

pub fn die_if_err<T, E: std::fmt::Display>(res: Result<T, E>) -> T {
    match res {
        Ok(t) => t,
        Err(e) => {
            die_err(&format!("ERROR: {}", e));
        }
    }
}
