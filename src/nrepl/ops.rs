use crate::bencode as bc;
use crate::nrepl;
use crate::nrepl::NreplOp;
use fs2::FileExt;
use serde_bencode::value::Value as BencodeValue;
use std::collections::HashMap;
use std::convert::From;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    NreplError(nrepl::Error),
    IOError(std::io::Error),
    NoSessionIdInResponse,
    NoSessionsInResponse,
    BadSessionIdValue(BencodeValue),
    JsonError(serde_json::error::Error),
    ToStringError(bc::Error),
}

impl From<serde_json::error::Error> for Error {
    fn from(e: serde_json::error::Error) -> Self {
        Self::JsonError(e)
    }
}

impl From<nrepl::Error> for Error {
    fn from(e: nrepl::Error) -> Self {
        Self::NreplError(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Self::IOError(e)
    }
}

impl From<bc::Error> for Error {
    fn from(e: bc::Error) -> Error {
        Self::ToStringError(e)
    }
}

pub struct CloneSession {
    pub session: Option<String>,
}

impl From<&CloneSession> for nrepl::Op {
    fn from(CloneSession { session }: &CloneSession) -> nrepl::Op {
        let mut args: Vec<(String, String)> = vec![];

        if let Some(s) = session {
            args.push(("session".to_string(), s.to_string()))
        }

        nrepl::Op::new("clone".to_string(), args)
    }
}

impl nrepl::NreplOp<String> for CloneSession {
    type Error = Error;

    fn send(&self, n: &nrepl::NreplStream) -> Result<String, Error> {
        for mut resp in n.op(self)? {
            if let Some(session_id) = resp.remove("new-session") {
                return Ok(bc::try_into_string(session_id)?);
            }
        }
        Err(Error::NoSessionIdInResponse)
    }
}

pub struct LsSessions {}

impl From<&LsSessions> for nrepl::Op {
    fn from(_op: &LsSessions) -> nrepl::Op {
        nrepl::Op::new("ls-sessions".to_string(), vec![])
    }
}

impl nrepl::NreplOp<Vec<String>> for LsSessions {
    type Error = Error;

    fn send(self: &LsSessions, n: &nrepl::NreplStream) -> Result<Vec<String>, Error> {
        for mut resp in n.op(self)? {
            if let Some(sessions) = resp.remove("sessions") {
                return Ok(bc::try_into_str_vec(sessions)?);
            }
        }
        Err(Error::NoSessionsInResponse)
    }
}

pub enum OpResp {
    CloneSession { new_session: String },
    LsSessions { sessions: Vec<String> },
}

fn create_session(nrepl: &nrepl::NreplStream) -> Result<String, Error> {
    let op = CloneSession { session: None };

    Ok(op.send(nrepl)?)
}

fn config_path() -> PathBuf {
    let mut dir = dirs::data_local_dir().unwrap();

    dir.push("ultra_nrepl");

    dir
}

fn sessions_path() -> PathBuf {
    let mut dir = config_path();

    dir.push("sessions.json");

    dir
}

fn save_session_id(n: &nrepl::NreplStream, session_id: &String) -> Result<(), Error> {
    let mut sessions: HashMap<String, String> = HashMap::new();

    std::fs::DirBuilder::new()
        .recursive(true)
        .create(config_path())?;

    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(sessions_path())?;

    f.lock_exclusive()?;

    if f.metadata()?.len() != 0 {
        sessions = serde_json::from_reader(&mut f)?;
    }

    sessions.insert(n.addr_string(), session_id.clone());

    f.set_len(0)?;
    f.write(&serde_json::to_string(&sessions)?.into_bytes())?;

    Ok(())
}

fn load_session_id(n: &nrepl::NreplStream) -> Result<Option<String>, Error> {
    let sid = std::fs::File::open(sessions_path())
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
