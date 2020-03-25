use crate::config;
use crate::nrepl;
use crate::nrepl::NreplOp;
use failure::{Error as StdError, Fail};
use nrepl::ops::{CloneSession, LsSessions};
use serde_bencode::value::Value as BencodeValue;

///! Module for maintaining persistent session-id within single nrepl connection

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "io error while managing session data: {}", ioerr)]
    /// Means that something is wrong when we're reading sessions file
    IOError { ioerr: std::io::Error },
    #[fail(display = "expected session id string, but had: {:?}", bencode)]
    /// When we've failed to read session from nrepl response (unlikely, but who knows!)
    BadSessionIdValue { bencode: BencodeValue },
    #[fail(display = "config error: {}", cfgerr)]
    ConfigError { cfgerr: config::Error },
}

impl From<std::io::Error> for Error {
    fn from(ioerr: std::io::Error) -> Error {
        Self::IOError { ioerr }
    }
}

impl From<config::Error> for Error {
    fn from(cfgerr: config::Error) -> Self {
        Self::ConfigError { cfgerr }
    }
}

fn create_session(nrepl: &nrepl::NreplStream) -> Result<String, StdError> {
    let op = CloneSession::new(None);

    Ok(op.send(nrepl)?)
}

fn save_session_id(n: &nrepl::NreplStream, session_id: &String) -> Result<(), StdError> {
    let session = config::Session::new(n.addr_string(), session_id.clone(), vec![]);
    config::save_session(session)?;

    Ok(())
}

fn load_session_id(n: &nrepl::NreplStream) -> Result<Option<String>, StdError> {
    let mb_session = config::load_session(n.addr_string())?.map(|s| s.session());
    Ok(mb_session)
}

fn session_id_exists(n: &nrepl::NreplStream, session_id: &String) -> Result<bool, StdError> {
    let op = LsSessions::new();

    for session in op.send(n)? {
        if &session == session_id {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Searches for a known session in nrepl otherwise creates a new one
pub fn get_existing_session_id(n: &nrepl::NreplStream) -> Result<String, StdError> {
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
