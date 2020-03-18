use failure::Fail;
use serde_bencode::value::Value;
use serde_json::value::Value as JsonValue;

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

pub fn try_into_int(val: Value) -> Result<i64, Error> {
    if let Value::Int(n) = val {
        Ok(n)
    } else {
        Err(Error::InvalidType {
            bc: format!("{:?}", val),
        })
    }
}

#[derive(Debug)]
pub enum JsonError {}

pub fn to_json_value(val: Value) -> Result<JsonValue, JsonError> {
    match val {
        Value::Bytes(bs) => Ok(serde_json::Value::String(String::from_utf8(bs).unwrap())),
        Value::List(items) => Ok(JsonValue::Array(
            items
                .into_iter()
                .map(|i| Ok(to_json_value(i)?))
                .collect::<Result<Vec<JsonValue>, JsonError>>()?,
        )),
        Value::Dict(hm) => {
            let m = hm
                .into_iter()
                .map(|(k, v)| Ok((String::from_utf8(k).unwrap(), to_json_value(v)?)))
                .collect::<Result<serde_json::Map<String, JsonValue>, JsonError>>()?;

            Ok(JsonValue::Object(m))
        }

        Value::Int(i) => Ok(JsonValue::Number(
            serde_json::Number::from_f64(i as f64).unwrap(),
        )),
    }
}
