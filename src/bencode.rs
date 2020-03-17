use failure::Fail;
use serde_bencode::value::Value;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "invalid bencode type: {}", bc)]
    InvalidType { bc: String },
    #[fail(display = "failed to parse utf8: {}", utf8err)]
    Utf8Error { utf8err: std::string::FromUtf8Error },
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Self::Utf8Error { utf8err: err }
    }
}

pub fn try_into_string(val: Value) -> Result<String, Error> {
    if let Value::Bytes(bs) = val {
        Ok(String::from_utf8(bs)?)
    } else {
        Err(Error::InvalidType {
            bc: format!("{:?}", val),
        })
    }
}

pub fn try_into_str_vec(val: Value) -> Result<Vec<String>, Error> {
    if let Value::List(vals) = val {
        vals.into_iter()
            .map(|v| try_into_string(v))
            .collect::<Result<Vec<String>, Error>>()
    } else {
        Err(Error::InvalidType {
            bc: format!("{:?}", val),
        })
    }
}
