use crate::bencode as bc;
use crate::nrepl;
use failure::Fail;
use std::convert::From;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "nrepl error: {}", nrepl_err)]
    NreplError { nrepl_err: nrepl::Error },
    #[fail(display = "no session id in response")]
    NoSessionIdInResponse,
    #[fail(display = "no sessions list in response")]
    NoSessionsInResponse,
    #[fail(display = "failed converting bencode to string: {}", bcerr)]
    ToStringError { bcerr: bc::Error },
    #[fail(display = "field was not found: {}", field)]
    FieldNotFound { field: String },
}

impl From<nrepl::Error> for Error {
    fn from(e: nrepl::Error) -> Self {
        Self::NreplError { nrepl_err: e }
    }
}

impl From<bc::Error> for Error {
    fn from(e: bc::Error) -> Error {
        Self::ToStringError { bcerr: e }
    }
}

pub struct CloneSession {
    session: Option<String>,
}

impl CloneSession {
    pub fn new(session: Option<String>) -> Self {
        Self { session }
    }
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

impl LsSessions {
    pub fn new() -> Self {
        Self {}
    }
}

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

pub struct Info {
    ns: String,
    symbol: String,
}

pub struct InfoResponse {
    line: u32,
    col: u32,
    file: String,
    resource: String,
    doc: String,
}

impl InfoResponse {
    pub fn new(line: u32, col: u32, file: String, resource: String, doc: String) -> Self {
        Self {
            line,
            col,
            file,
            resource,
            doc,
        }
    }
}

impl Info {
    pub fn new(ns: String, symbol: String) -> Self {
        Self { ns, symbol }
    }
}

impl From<&Info> for nrepl::Op {
    fn from(Info { ns, symbol }: &Info) -> nrepl::Op {
        nrepl::Op::new(
            "info".to_string(),
            vec![
                ("symbol".to_string(), symbol.to_string()),
                ("ns".to_string(), ns.to_string()),
            ],
        )
    }
}

// impl nrepl::NreplOp<InfoResponse> for Info {
//     type Error = Error;

//     fn send(self: &Info, n: &nrepl::NreplStream) -> Result<InfoResponse, Error> {
//         let mut resps = n.op(self)?;
//         let resp = resps.pop().unwrap();
//         let line: Result<i64, Self::Error> = resp
//             .remove("line")
//             .ok_or(Err(Error::FieldNotFound {
//                 field: "line".to_string(),
//             })).and_then(|n| {
//                 bc::try_into_int(n)
//             });

//         // InfoResponse::new(

//         // );

//         Err(Error::NoSessionsInResponse)
//     }
// }
