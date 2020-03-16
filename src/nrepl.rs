pub mod ops;
pub mod session;

// use crate::bencode::json as bencode_json;
use bendy::encoding::{Error as BError, SingleItemEncoder, ToBencode};
use serde::{Deserialize, Serialize};
use serde_bencode::value::Value as BencodeValue;
use std::collections::HashMap;
use std::convert::Into;
use std::convert::TryFrom;
use std::fmt;
use std::io::{BufWriter, Write};
use std::iter::FromIterator;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

#[derive(Debug)]
pub enum Error {
    ConnectionLost,
    BencodeEncodeError(BError),
    IOError(std::io::Error),
    BadBencodeString(std::string::FromUtf8Error),
    DuplicatedKeyError(String),
    BencodeDeserializeError(serde_bencode::error::Error),
    BencodeFormatError(RespError),
}

impl std::convert::From<serde_bencode::error::Error> for Error {
    fn from(err: serde_bencode::error::Error) -> Self {
        Self::BencodeDeserializeError(err)
    }
}

impl std::convert::From<RespError> for Error {
    fn from(err: RespError) -> Self {
        Self::BencodeFormatError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Nrepl Error: {}",
            match self {
                Error::ConnectionLost => format!("Connection lost!"),
                Error::IOError(io_err) => format!("IO Error: {}", io_err),
                Error::BencodeEncodeError(berr) => format!("Bencode encode error: {}", berr),
                Error::BadBencodeString(utf8err) => format!("Bad string in bencode: {}", utf8err),
                Error::DuplicatedKeyError(k) =>
                    format!("Key {} was duplicated in response dict", k),
                Error::BencodeDeserializeError(e) =>
                    format!("Failed to deserialize bencode: {}", e),
                Error::BencodeFormatError(e) => format!("Bad format of bencode: {}", e),
            }
        )
    }
}

pub struct NreplStream {
    tcp: TcpStream,
    socket_addr: SocketAddr,
}

pub struct Op {
    name: String,
    args: Vec<(String, String)>,
}

impl Op {
    pub fn new(name: String, args: Vec<(String, String)>) -> Op {
        Op { name, args }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Resp(HashMap<String, BencodeValue>);

// #[derive(Debug, Deserialize, Serialize)]
// pub enum RespVal {
//     Str(String),
//     Array(Vec<String>),
// }

#[derive(Debug)]
pub enum RespError {
    ExpectedMap(BencodeValue),
    ExpectedString(BencodeValue),
    ExpectedStrOrArray(BencodeValue),
    BadUtf8(std::string::FromUtf8Error),
}

// impl std::convert::From<&RespVal> for JsonValue {
//     fn from(v: &RespVal) -> JsonValue {
//         match v {
//             RespVal::Str(s) => JsonValue::String(s.clone()),
//             RespVal::Array(ls) => JsonValue::Array(
//                 ls.into_iter()
//                     .map(|v| JsonValue::String(v.clone()))
//                     .collect(),
//             ),
//         }
//     }
// }

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

// impl TryFrom<BencodeValue> for RespVal {
//     type Error = RespError;

//     fn try_from(val: BencodeValue) -> Result<Self, Self::Error> {
//         match val {
//             BencodeValue::Bytes(bs) => Ok(Self::Str(String::from_utf8(bs)?)),
//             BencodeValue::List(ls) => {
//                 let vals = ls
//                     .into_iter()
//                     .map(|v| match v {
//                         BencodeValue::Bytes(bs) => Ok(String::from_utf8(bs)?),
//                         v => Err(Self::Error::ExpectedString(v)),
//                     })
//                     .collect::<Result<Vec<String>, Self::Error>>()?;
//                 Ok(Self::Array(vals))
//             }
//             v => Err(Self::Error::ExpectedStrOrArray(v)),
//         }
//     }
// }

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

impl ToBencode for Op {
    const MAX_DEPTH: usize = 3;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), BError> {
        encoder.emit_dict(|mut e| {
            let mut pairs: Vec<(&str, &str)> = vec![];

            pairs.push(("op", &self.name));

            for (argname, argval) in self.args.iter() {
                pairs.push((argname, argval));
            }

            pairs.sort();

            for (argname, argval) in pairs.into_iter() {
                e.emit_pair(&argname.clone().as_bytes(), argval)?;
            }

            Ok(())
        })?;
        Ok(())
    }
}

fn is_final_resp(resp: &Resp) -> bool {
    resp.contains_key("status")
}

/// It is responsible for communication with nrepl bencode socket

impl NreplStream {
    pub fn connect_timeout(addr: &SocketAddr) -> Result<NreplStream, Error> {
        TcpStream::connect_timeout(addr, Duration::new(3, 0))
            .and_then(|t| {
                t.set_nonblocking(false)?;
                t.set_read_timeout(Some(Duration::new(5, 0)))?;
                Ok(t)
            })
            .map(|s| NreplStream {
                tcp: s,
                socket_addr: addr.clone(),
            })
            .map_err(|e| Error::IOError(e))
    }

    fn send_op<T: Into<Op>>(&self, op: T) -> Result<(), Error> {
        let mut bw = BufWriter::new(&self.tcp);
        let bencode = op
            .into()
            .to_bencode()
            .map_err(|e| Error::BencodeEncodeError(e))?;
        bw.write(&bencode).map_err(|e| Error::IOError(e))?;
        Ok(())
    }

    fn read_resp(&self) -> Result<Resp, Error> {
        let mut deser = serde_bencode::de::Deserializer::new(&self.tcp);

        let val: BencodeValue = serde::Deserialize::deserialize(&mut deser).unwrap();

        Ok(TryFrom::try_from(val)?)
    }

    /// Serializes given `op` and sends to Nrepl socket using given transport
    pub fn op<T: Into<Op>>(&self, op: T) -> Result<Vec<Resp>, Error> {
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

        Ok(resps)
    }

    pub fn addr_string(&self) -> String {
        self.socket_addr.to_string()
    }
}

pub trait NreplOp<T> {
    type Error;

    fn send(&self, nrepl: &NreplStream) -> Result<T, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bencode;
    use std::collections::HashMap;
    use std::iter::FromIterator;

    #[test]
    fn final_resp_test() {
        let final_resp = HashMap::from_iter(
            vec![("status".to_string(), bencode::Object::BBytes(vec![]))].into_iter(),
        );

        let not_final_resp = HashMap::from_iter(
            vec![("foo".to_string(), bencode::Object::BBytes(vec![]))].into_iter(),
        );

        assert!(is_final_resp(&final_resp));
        assert!(!is_final_resp(&not_final_resp));
    }
}
