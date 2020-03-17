use std::collections::HashMap;
use std::convert::From;
use std::path::PathBuf;

#[derive(Debug, failure::Fail)]
pub enum Error {
    #[fail(display = "failed to parse sessions: {}", error)]
    SessionsParseError { error: serde_json::Error },

    #[fail(display = "had problems with reading sessions file: {}", ioerr)]
    SessionsReadError { ioerr: std::io::Error },
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::SessionsParseError { error }
    }
}

impl From<std::io::Error> for Error {
    fn from(ioerr: std::io::Error) -> Self {
        Self::SessionsReadError { ioerr }
    }
}

pub fn config_path() -> PathBuf {
    let mut dir = dirs::data_local_dir().unwrap();

    dir.push("ultra_nrepl");

    dir
}

pub fn sessions_path() -> PathBuf {
    let mut dir = config_path();

    dir.push("sessions.json");

    dir
}

pub fn ensure_config_dir() -> Result<(), std::io::Error> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(config_path())?;

    Ok(())
}

pub fn parse_sessions(f: &mut std::fs::File) -> Result<HashMap<String, String>, Error> {
    if f.metadata()?.len() == 0 {
        Ok(HashMap::new())
    } else {
        Ok(serde_json::from_reader(f)?)
    }
}
