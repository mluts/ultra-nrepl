use crate::config;
use crate::nrepl;
use crate::nrepl::NreplOp;
use failure::Fail;
use fs2::FileExt;
use nrepl::ops;
use nrepl::ops::{CloneSession, LsSessions};
use serde_bencode::value::Value as BencodeValue;
use std::collections::HashMap;
use std::io::{Seek, Write};

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "io error while managing session data: {}", ioerr)]
    IOError { ioerr: std::io::Error },
    #[fail(display = "expected session id string, but had: {:?}", bencode)]
    BadSessionIdValue { bencode: BencodeValue },
    #[fail(display = "failed to parse sessions json")]
    JsonError(serde_json::error::Error),
    #[fail(display = "op error: {}", operr)]
    OpError { operr: ops::Error },
    #[fail(display = "config error: {}", cfgerr)]
    ConfigError { cfgerr: config::Error },
}

impl From<serde_json::error::Error> for Error {
    fn from(e: serde_json::error::Error) -> Self {
        Self::JsonError(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(ioerr: std::io::Error) -> Error {
        Self::IOError { ioerr }
    }
}

impl From<ops::Error> for Error {
    fn from(operr: ops::Error) -> Self {
        Self::OpError { operr }
    }
}

impl From<config::Error> for Error {
    fn from(cfgerr: config::Error) -> Self {
        Self::ConfigError { cfgerr }
    }
}

fn create_session(nrepl: &nrepl::NreplStream) -> Result<String, Error> {
    let op = CloneSession { session: None };

    Ok(op.send(nrepl)?)
}

fn save_session_id(n: &nrepl::NreplStream, session_id: &String) -> Result<(), Error> {
    config::ensure_config_dir()?;

    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(config::sessions_path())?;

    f.lock_exclusive()?;

    let mut sessions = config::parse_sessions(&mut f)?;

    sessions.insert(n.addr_string(), session_id.clone());

    f.set_len(0)?;
    f.seek(std::io::SeekFrom::Start(0))?;
    f.write(&serde_json::to_string(&sessions)?.into_bytes())?;

    Ok(())
}

fn load_session_id(n: &nrepl::NreplStream) -> Result<Option<String>, Error> {
    let sid = std::fs::File::open(config::sessions_path())
        .and_then(|mut f| {
            if f.metadata()?.len() > 0 {
                let mut sessions: HashMap<String, String> = serde_json::from_reader(&mut f)?;
                Ok(sessions.remove(&n.addr_string()))
            } else {
                Ok(None)
            }
        })
        .or_else(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Ok(None),
            _ => Err(e),
        })?;

    Ok(sid)
}

fn session_id_exists(n: &nrepl::NreplStream, session_id: &String) -> Result<bool, Error> {
    let op = LsSessions {};

    for session in op.send(n)? {
        if &session == session_id {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn get_existing_session_id(n: &nrepl::NreplStream) -> Result<String, Error> {
    let mb_session_id = load_session_id(n)?;

    if let Some(existing_session_id) = mb_session_id {
        if session_id_exists(n, &existing_session_id)? {
            return Ok(existing_session_id);
        }
    }

    let new_session_id = create_session(n)?;

    save_session_id(n, &new_session_id)?;

    Ok(new_session_id)
}
