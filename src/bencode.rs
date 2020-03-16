use serde_bencode::value::Value;

#[derive(Debug)]
pub enum Error {
    InvalidType(String),
    Utf8Error(std::string::FromUtf8Error),
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

pub fn try_into_string(val: Value) -> Result<String, Error> {
    if let Value::Bytes(bs) = val {
        Ok(String::from_utf8(bs)?)
    } else {
        Err(Error::InvalidType(format!("{:?}", val)))
    }
}

pub fn try_into_str_vec(val: Value) -> Result<Vec<String>, Error> {
    if let Value::List(vals) = val {
        vals.into_iter()
            .map(|v| try_into_string(v))
            .collect::<Result<Vec<String>, Error>>()
    } else {
        Err(Error::InvalidType(format!("{:?}", val)))
    }
}
