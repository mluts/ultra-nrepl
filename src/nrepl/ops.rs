use crate::bencode as bc;
use crate::nrepl;
use failure::{Error as StdError, Fail};
use serde::Serialize;
use std::convert::From;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "nrepl error when sending op: {}", nrepl_err)]
    NreplError { nrepl_err: nrepl::Error },
    #[fail(display = "sent `{}`, but no session id in response", op)]
    NoSessionIdInResponse { op: String },
    #[fail(display = "Sent `{}`, but no sessions list in response", op)]
    NoSessionsInResponse { op: String },
    #[fail(display = "failed converting bencode when decoded op: {}", bcerr)]
    BencodeConvertError { bcerr: bc::Error },
    #[fail(
        display = "Sent `{}`, expected to find field `{}`, but it wasn't in nrepl response",
        op, field
    )]
    FieldNotFound { op: String, field: String },
    #[fail(display = "Unexpected nrepl status: {}", status)]
    BadStatus { status: String },
}

impl From<nrepl::Error> for Error {
    fn from(e: nrepl::Error) -> Self {
        Self::NreplError { nrepl_err: e }
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
    type Error = StdError;

    fn send(&self, n: &nrepl::NreplStream) -> Result<String, StdError> {
        match n.op(self)? {
            nrepl::Status::Done(resps) => {
                for mut resp in resps {
                    if let Some(session_id) = resp.remove("new-session") {
                        return Ok(bc::try_into_string(session_id)?);
                    }
                }
                return Err(Error::NoSessionIdInResponse {
                    op: "clone".to_string(),
                }
                .into());
            }
            status => Err(Error::BadStatus {
                status: status.name(),
            }
            .into()),
        }
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
    type Error = StdError;

    fn send(self: &LsSessions, n: &nrepl::NreplStream) -> Result<Vec<String>, Self::Error> {
        match n.op(self)? {
            nrepl::Status::Done(resps) => {
                for mut resp in resps {
                    if let Some(sessions) = resp.remove("sessions") {
                        return Ok(bc::try_into_str_vec(sessions)?);
                    }
                }
                return Err(Error::NoSessionsInResponse {
                    op: "ls-sessions".to_string(),
                }
                .into());
            }

            status => Err(Error::BadStatus {
                status: status.name(),
            }
            .into()),
        }
    }
}

pub struct Info {
    ns: String,
    symbol: String,
    session: String,
}

#[derive(Debug, Serialize)]
pub struct InfoResponse {
    pub line: i64,
    pub col: Option<i64>,
    pub file: String,
    pub resource: String,
    pub doc: String,
}

pub enum InfoResponseType {
    Ns(InfoResponse),
    Symbol(InfoResponse),
}

impl InfoResponseType {
    pub fn into_resp(self) -> InfoResponse {
        match self {
            Self::Ns(r) => r,
            Self::Symbol(r) => r,
        }
    }
}

impl InfoResponse {
    pub fn new(line: i64, col: Option<i64>, file: String, resource: String, doc: String) -> Self {
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
    pub fn new(session: String, ns: String, symbol: String) -> Self {
        Self {
            session,
            ns,
            symbol,
        }
    }
}

impl From<&Info> for nrepl::Op {
    fn from(
        Info {
            session,
            ns,
            symbol,
        }: &Info,
    ) -> nrepl::Op {
        nrepl::Op::new(
            "info".to_string(),
            vec![
                ("symbol".to_string(), symbol.to_string()),
                ("ns".to_string(), ns.to_string()),
                ("session".to_string(), session.to_string()),
            ],
        )
    }
}

fn get_int_bencode(resp: &mut nrepl::Resp, k: &str) -> Result<Option<i64>, StdError> {
    if let Some(n) = resp.remove(k) {
        Ok(Some(bc::try_into_int(n)?))
    } else {
        Ok(None)
    }
}

fn get_str_bencode(resp: &mut nrepl::Resp, k: &str) -> Result<Option<String>, StdError> {
    if let Some(s) = resp.remove(k) {
        Ok(Some(bc::try_into_string(s)?))
    } else {
        Ok(None)
    }
}

impl nrepl::NreplOp<Option<InfoResponseType>> for Info {
    type Error = StdError;

    fn send(self: &Info, n: &nrepl::NreplStream) -> Result<Option<InfoResponseType>, Self::Error> {
        match n.op(self)? {
            nrepl::Status::Done(mut resps) => {
                let mut resp = resps.pop().unwrap();
                let line: Option<i64> = get_int_bencode(&mut resp, "line")?;
                let column: Option<i64> = get_int_bencode(&mut resp, "column")?;

                if let Some(v) = resp.get("file") {
                    if let serde_bencode::value::Value::List(_) = v {
                        return Ok(None);
                    }
                }

                let file: String =
                    get_str_bencode(&mut resp, "file")?.ok_or(Error::FieldNotFound {
                        op: "info".to_string(),
                        field: "file".to_string(),
                    })?;
                let resource: String =
                    get_str_bencode(&mut resp, "resource")?.ok_or(Error::FieldNotFound {
                        op: "info".to_string(),
                        field: "resource".to_string(),
                    })?;

                let doc: Option<String> = get_str_bencode(&mut resp, "doc")?;
                let name: Option<String> = get_str_bencode(&mut resp, "name")?;
                let arglist: Option<String> = get_str_bencode(&mut resp, "arglists-str")?;
                let ns: Option<String> = get_str_bencode(&mut resp, "ns")?;

                if line.is_some() && column.is_none() && name.is_none() && arglist.is_none() {
                    Ok(Some(InfoResponseType::Ns(InfoResponse::new(
                        line.unwrap(),
                        column,
                        file,
                        resource,
                        format!(
                            "{}\n{}",
                            &ns.unwrap_or("".to_string()),
                            &doc.unwrap_or("".to_string())
                        ),
                    ))))
                } else {
                    Ok(Some(InfoResponseType::Symbol(InfoResponse::new(
                        line.unwrap(),
                        column,
                        file,
                        resource,
                        format!(
                            "{}/{}\n{}\n{}",
                            &ns.unwrap_or("".to_string()),
                            &name.unwrap_or("".to_string()),
                            &arglist
                                .unwrap_or("".to_string())
                                .split("\n")
                                .map(|s| format!("({})", s))
                                .collect::<Vec<String>>()
                                .join("\n"),
                            &doc.unwrap_or("".to_string()),
                        ),
                    ))))
                }
            }

            nrepl::Status::NoInfo(_) => Ok(None),
            status => Err(Error::BadStatus {
                status: status.name(),
            }
            .into()),
        }
    }
}

pub struct GetNsName {
    source_path: String,
}

impl GetNsName {
    pub fn new(source_path: String) -> Self {
        Self { source_path }
    }
}

impl From<&GetNsName> for nrepl::Op {
    fn from(GetNsName { source_path }: &GetNsName) -> nrepl::Op {
        nrepl::Op::new(
            "eval".to_string(),
            vec![(
                "code".to_string(),
                format!(
                    "
             (do
                (require 'clojure.tools.namespace.file)
                (nth (clojure.tools.namespace.file/read-file-ns-decl \"{}\") 1)
             )",
                    source_path
                ),
            )],
        )
    }
}

impl nrepl::NreplOp<Option<String>> for GetNsName {
    type Error = StdError;

    fn send(&self, n: &nrepl::NreplStream) -> Result<Option<String>, Self::Error> {
        match n.op(self)? {
            nrepl::Status::Done(resps) => {
                let mut value: Option<String> = None;

                for mut resp in resps {
                    if let Some(val) = resp.remove("value") {
                        value = Some(bc::try_into_string(val)?)
                    }
                }
                Ok(value)
            }

            status => Err(Error::BadStatus {
                status: status.name(),
            }
            .into()),
        }
    }
}
