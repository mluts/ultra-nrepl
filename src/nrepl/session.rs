use crate::config;
use crate::config::Session;
use crate::nrepl;
use crate::nrepl::NreplOp;
use failure::{Error as StdError, Fail};
use nrepl::ops::{CloneSession, Describe, LsSessions};
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

fn create_session(nrepl: &nrepl::NreplStream) -> Result<Session, StdError> {
    let id = CloneSession::new(None).send(nrepl)?;
    let describe = Describe::new(false).send(nrepl)?;

    Ok(Session::new(
        nrepl.addr_string(),
        id,
        describe.into_ops().into_iter().collect(),
    ))
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
pub fn get_existing_session_id(n: &nrepl::NreplStream) -> Result<Session, StdError> {
    let mb_session = config::load_session(n.addr_string())?;

    if let Some(existing_session) = mb_session {
        if session_id_exists(n, &existing_session.id())? {
            return Ok(existing_session);
        }
    }

    let new_session = create_session(n)?;

    config::save_session(&new_session)?;

    Ok(new_session)
}
