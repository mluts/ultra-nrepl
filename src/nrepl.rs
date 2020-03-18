pub mod ops;
pub mod session;

use crate::bencode;
use failure::Fail;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use serde_bencode::value::Value as BencodeValue;
use std::collections::HashMap;
use std::convert::{From, Into, TryFrom};
use std::fmt;
use std::io::{BufWriter, Write};
use std::iter::FromIterator;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "nrepl io error: {}", ioerr)]
    IOError { ioerr: std::io::Error },
    #[fail(display = "bencode string decode failed: {}", utf8err)]
    BadBencodeString { utf8err: std::string::FromUtf8Error },
    #[fail(display = "bencode deserialize failed: {}", bencode_err)]
    BencodeDeserializeError {
        bencode_err: serde_bencode::error::Error,
    },
    #[fail(display = "Bencode format error")]
    BencodeFormatError(RespError),
    #[fail(display = "Nrepl returned unsuccessful status: {}", status)]
    ResponseStatusError { status: String },
}

pub enum Status {
    Done(Vec<Resp>),
    NoInfo(Vec<Resp>),
    EvalError(Vec<Resp>),
    UnknownStatus(Vec<String>, Vec<Resp>),
}

impl Status {
    pub fn name(&self) -> String {
        match self {
            Self::Done(_) => "done".to_string(),
            Self::NoInfo(_) => "no-info".to_string(),
            Self::EvalError(_) => "eval-error".to_string(),
            Self::UnknownStatus(statuses, _) => statuses.join(","),
        }
    }

    pub fn into_resps(self) -> Vec<Resp> {
        match self {
            Self::Done(resps) => resps,
            Self::NoInfo(resps) => resps,
            Self::EvalError(resps) => resps,
            Self::UnknownStatus(_, resps) => resps,
        }
    }
}

impl From<serde_bencode::error::Error> for Error {
    fn from(bencode_err: serde_bencode::error::Error) -> Self {
        Self::BencodeDeserializeError { bencode_err }
    }
}

impl From<std::io::Error> for Error {
    fn from(ioerr: std::io::Error) -> Self {
        Self::IOError { ioerr }
    }
}

impl From<RespError> for Error {
    fn from(err: RespError) -> Self {
        Self::BencodeFormatError(err)
    }
}

pub struct NreplStream {
    tcp: TcpStream,
    socket_addr: SocketAddr,
}

#[derive(Debug)]
pub struct Op {
    name: String,
    args: Vec<(String, String)>,
}

impl Op {
    pub fn new(name: String, args: Vec<(String, String)>) -> Op {
        Op { name, args }
    }
}

impl Serialize for Op {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = s.serialize_map(Some(1 + self.args.len()))?;

        state.serialize_entry("op", &self.name)?;

        for (k, v) in self.args.iter() {
            state.serialize_entry(k, v)?;
        }

        state.end()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Resp(HashMap<String, BencodeValue>);

#[derive(Debug)]
pub enum RespError {
    ExpectedMap(BencodeValue),
    ExpectedString(BencodeValue),
    ExpectedStrOrArray(BencodeValue),
    BadUtf8(std::string::FromUtf8Error),
}

impl std::convert::From<std::string::FromUtf8Error> for RespError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Self::BadUtf8(err)
    }
}

impl fmt::Display for RespError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RespError::ExpectedMap(v) => {
                    format!(
                        "Expected dict, instead found: {}",
                        serde_json::to_string(v).unwrap()
                    )
                }

                RespError::ExpectedStrOrArray(v) => {
                    format!("Expected str or array, found: {:?}", v)
                }

                RespError::BadUtf8(_) => "Bencode string was broken".to_string(),

                RespError::ExpectedString(v) => format!("Expected string, found: {:?}", v),
            }
        )
    }
}

impl std::ops::Deref for Resp {
    type Target = HashMap<String, BencodeValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Resp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TryFrom<BencodeValue> for Resp {
    type Error = RespError;

    fn try_from(val: BencodeValue) -> Result<Self, Self::Error> {
        match val {
            BencodeValue::Dict(map) => {
                let pairs = map
                    .into_iter()
                    .map(|(k, v)| (String::from_utf8(k).unwrap(), TryFrom::try_from(v).unwrap()));
                Ok(Self(HashMap::from_iter(pairs)))
            }
            v => Err(Self::Error::ExpectedMap(v)),
        }
    }
}

fn is_final_resp(resp: &Resp) -> bool {
    resp.contains_key("status")
}

fn get_status(resp: &Resp) -> Option<Vec<String>> {
    if let Some(status) = resp.get("status") {
        Some(bencode::try_into_str_vec(status.clone()).unwrap())
    } else {
        None
    }
}

fn parse_resps(resps: Vec<Resp>) -> Result<Status, Error> {
    for resp in resps.iter() {
        if is_final_resp(&resp) {
            let status = get_status(&resp).unwrap();

            if status == ["done"] {
                return Ok(Status::Done(resps));
            } else if status == ["eval-error"] {
                return Ok(Status::EvalError(resps));
            } else if status == ["done", "no-info"] {
                return Ok(Status::NoInfo(resps));
            } else {
                return Ok(Status::UnknownStatus(status, resps));
            }
        }
    }
    unreachable!()
}

/// It is responsible for communication with nrepl bencode socket

impl NreplStream {
    pub fn connect_timeout(addr: &SocketAddr) -> Result<NreplStream, Error> {
        let conn = TcpStream::connect_timeout(addr, Duration::new(3, 0))
            .and_then(|t| {
                t.set_nonblocking(false)?;
                t.set_read_timeout(Some(Duration::new(5, 0)))?;
                Ok(t)
            })
            .map(|s| NreplStream {
                tcp: s,
                socket_addr: addr.clone(),
            })?;
        Ok(conn)
    }

    fn send_op<T: Into<Op>>(&self, op: T) -> Result<(), Error> {
        let mut bw = BufWriter::new(&self.tcp);
        let bencode = serde_bencode::to_bytes(&op.into())?;
        bw.write(&bencode)?;
        Ok(())
    }

    fn read_resp(&self) -> Result<Resp, Error> {
        let mut deser = serde_bencode::de::Deserializer::new(&self.tcp);

        let val: BencodeValue = serde::Deserialize::deserialize(&mut deser).unwrap();

        Ok(TryFrom::try_from(val)?)
    }

    /// Serializes given `op` and sends to Nrepl socket using given transport
    pub fn op<T: Into<Op>>(&self, op: T) -> Result<Status, Error> {
        let mut resps: Vec<Resp> = vec![];

        self.send_op(op)?;

        loop {
            let resp = self.read_resp()?;
            let is_final = is_final_resp(&resp);

            resps.push(resp);

            if is_final {
                break;
            }
        }

        parse_resps(resps)
    }

    pub fn addr_string(&self) -> String {
        self.socket_addr.to_string()
    }
}

pub trait NreplOp<T> {
    type Error;

    fn send(&self, nrepl: &NreplStream) -> Result<T, Self::Error>;
}

pub fn default_nrepl_port() -> Option<u32> {
    std::fs::read_to_string(".nrepl-port")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
}

pub fn port_addr(port: u32) -> SocketAddr {
    format!("127.0.0.1:{}", port).parse().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_bencode::value::Value as BencodeValue;
    use std::collections::HashMap;
    use std::iter::FromIterator;

    #[test]
    fn final_resp_test() {
        let final_resp = Resp(HashMap::from_iter(
            vec![("status".to_string(), BencodeValue::Bytes(vec![]))].into_iter(),
        ));

        let not_final_resp = Resp(HashMap::from_iter(
            vec![("foo".to_string(), BencodeValue::Bytes(vec![]))].into_iter(),
        ));

        assert!(is_final_resp(&final_resp));
        assert!(!is_final_resp(&not_final_resp));
    }
}
