use crate::bencode as bc;
use crate::config::Session;
use crate::nrepl;
use failure::{Error as StdError, Fail};
use serde::Serialize;
use serde_bencode::value::Value as BencodeValue;
use std::collections::HashSet;
use std::convert::From;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "sent `{}`, but no session id in response", op)]
    NoSessionIdInResponse { op: String },
    #[fail(display = "Sent `{}`, but no sessions list in response", op)]
    NoSessionsInResponse { op: String },
    #[fail(
        display = "Sent `{}`, expected to find field `{}`, but it wasn't in nrepl response",
        op, field
    )]
    FieldNotFound { op: String, field: String },
    #[fail(display = "Unexpected nrepl status: {}", status)]
    BadStatus { status: String },
    #[fail(display = "Having two 'ops' dicts in response to 'describe' op")]
    DuplicatedOpsInResponse,
    #[fail(display = "'info' op is not available")]
    InfoOpUnavailable,
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
    session: Session,
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
    pub fn new(session: Session, ns: String, symbol: String) -> Self {
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
                ("session".to_string(), session.id()),
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

fn get_str_list_bencode(resp: &mut nrepl::Resp, k: &str) -> Result<Option<Vec<String>>, StdError> {
    if let Some(sl) = resp.remove(k) {
        Ok(Some(bc::try_into_str_vec(sl)?))
    } else {
        Ok(None)
    }
}
impl nrepl::NreplOp<Option<InfoResponseType>> for Info {
    type Error = StdError;

    // This function is quite bulky, but currently i don't see (or i am not interested in) how to
    // organize it better.
    // I wanted to have a greater control under parsing SYMBOL/NS/JavaClass
    fn send(self: &Info, n: &nrepl::NreplStream) -> Result<Option<InfoResponseType>, Self::Error> {
        if !self.session.is_op_available("info") {
            return Err(Error::InfoOpUnavailable.into());
        }

        match n.op(self)? {
            nrepl::Status::Done(mut resps) | nrepl::Status::State(mut resps) => {
                let mut resp = resps.pop().unwrap();
                // "line" is required for symbols, but not namespace, TODO: Improve this
                let line: Option<i64> = get_int_bencode(&mut resp, "line")?;
                let column: Option<i64> = get_int_bencode(&mut resp, "column")?;

                // It's weird, but valid:
                // When we received {file: [...]} it means that given given symbol was a java class,
                // and we have nothing to do with Java Class here.
                if let Some(v) = resp.get("file") {
                    if let serde_bencode::value::Value::List(_) = v {
                        return Ok(None);
                    }
                }

                // This is required field, we can't skip it
                let file: String =
                    get_str_bencode(&mut resp, "file")?.ok_or(Error::FieldNotFound {
                        op: "info".to_string(),
                        field: "file".to_string(),
                    })?;

                // Actually, resource is not mandatory, TODO: Improve this
                let resource: String =
                    get_str_bencode(&mut resp, "resource")?.ok_or(Error::FieldNotFound {
                        op: "info".to_string(),
                        field: "resource".to_string(),
                    })?;

                let doc: Option<String> = get_str_bencode(&mut resp, "doc")?;
                let name: Option<String> = get_str_bencode(&mut resp, "name")?;
                let arglist: Option<String> = get_str_bencode(&mut resp, "arglists-str")?;
                let ns: Option<String> = get_str_bencode(&mut resp, "ns")?;
                // We are only interested in presence of this field, not it's content
                // TODO: Check if "macro" could be other than "true"
                let is_macro: Option<String> = get_str_bencode(&mut resp, "macro")?;
                let spec: Option<String> =
                    get_str_list_bencode(&mut resp, "spec")?.map(|spec_list| spec_list.join(" "));
                let docstr: String;

                // There's only single way to distinguish NS from SYMBOL is by absence of
                // column/name/arglist
                if line.is_some() && column.is_none() && name.is_none() && arglist.is_none() {
                    docstr = vec![ns, doc]
                        .into_iter()
                        .flat_map(|v| v)
                        .collect::<Vec<String>>()
                        .join("\n");

                    Ok(Some(InfoResponseType::Ns(InfoResponse::new(
                        line.unwrap(),
                        column,
                        file,
                        resource,
                        docstr,
                    ))))
                // Otherwise it's SYMBOL
                } else {
                    docstr = vec![
                        String::from(if is_macro.is_some() { "macro" } else { "" }),
                        vec![ns, name]
                            .into_iter()
                            .flat_map(|v| v)
                            .collect::<Vec<String>>()
                            .join("/"),
                        arglist
                            .unwrap_or("".to_string())
                            .split("\n")
                            .map(|s| format!("({})", s))
                            .collect::<Vec<String>>()
                            .join("\n"),
                        doc.unwrap_or(String::new()),
                        spec.unwrap_or(String::new()),
                    ]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<String>>()
                    .join("\n");

                    Ok(Some(InfoResponseType::Symbol(InfoResponse::new(
                        line.unwrap(),
                        column,
                        file,
                        resource,
                        docstr,
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

/// This OP is for parsing NS name from clojure file using clojure.tools.namespace
/// Using `eval` OP under hood
pub struct GetNsName {
    source_path: String,
    session: Session,
}

impl GetNsName {
    pub fn new(source_path: String, session: Session) -> Self {
        Self {
            source_path,
            session,
        }
    }
}

impl From<&GetNsName> for nrepl::Op {
    fn from(
        GetNsName {
            source_path,
            session,
        }: &GetNsName,
    ) -> nrepl::Op {
        nrepl::Op::new(
            "eval".to_string(),
            vec![
                (
                    "code".to_string(),
                    format!(
                        "
             (do
                (require 'clojure.tools.namespace.file)
                (nth (clojure.tools.namespace.file/read-file-ns-decl \"{}\") 1)
             )",
                        source_path
                    ),
                ),
                ("session".to_string(), session.id()),
            ],
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

pub struct Describe {
    verbose: bool,
}

impl Describe {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

pub struct DescribeResp {
    ops: HashSet<String>,
}

impl DescribeResp {
    pub fn ops(&self) -> &HashSet<String> {
        &self.ops
    }

    pub fn into_ops(self) -> HashSet<String> {
        self.ops
    }
}

impl From<&Describe> for nrepl::Op {
    fn from(Describe { verbose }: &Describe) -> nrepl::Op {
        let mut args: Vec<(String, String)> = vec![];
        if *verbose {
            args.push(("verbose?".to_string(), "true".to_string()));
        }
        nrepl::Op::new("describe".to_string(), args)
    }
}

impl nrepl::NreplOp<DescribeResp> for Describe {
    type Error = StdError;

    fn send(&self, n: &nrepl::NreplStream) -> Result<DescribeResp, Self::Error> {
        match n.op(self)? {
            nrepl::Status::Done(resps) | nrepl::Status::State(resps) => {
                let mut ops: Option<HashSet<String>> = None;

                for mut resp in resps {
                    if let Some(json_val) = resp.remove("ops") {
                        if let BencodeValue::Dict(ops_map) = json_val {
                            if ops.is_some() {
                                return Err(Error::DuplicatedOpsInResponse.into());
                            }
                            ops = Some(
                                ops_map
                                    .into_iter()
                                    .map(|(k, _)| String::from_utf8(k).map_err(|e| e.into()))
                                    .collect::<Result<HashSet<String>, Self::Error>>()?,
                            );
                        }
                    }
                }

                Ok(DescribeResp { ops: ops.unwrap() })
            }

            status => Err(Error::BadStatus {
                status: status.name(),
            }
            .into()),
        }
    }
}
