//! Transforms `bencode::Object` to `serde_json::value::Value`

use crate::bencode::Object;
use serde_json::map::Map;
use serde_json::value::Value;

#[derive(Debug)]
pub enum Error {
    DictKeyDecodeError(std::string::FromUtf8Error),
    StringDecodeError(std::string::FromUtf8Error),
    UnknownError,
}

/// # Example
///
/// ```
/// use ultra_nrepl::bencode::Object;
/// use ultra_nrepl::bencode::json::to_json_val;
/// use serde_json::value::Value;
///
/// assert_eq!(
///     Value::Array(vec![Value::String("foobar".to_string())]),
///     to_json_val(&Object::List(vec![Object::BBytes(b"foobar".to_vec())])).unwrap()
/// )
/// ```

pub fn to_json_val(obj: &Object) -> Result<Value, Error> {
    match obj {
        Object::List(objs) => {
            let mut array_vals: Vec<Value> = vec![];
            for obj in objs.into_iter() {
                let v = to_json_val(obj)?;
                array_vals.push(v);
            }

            return Ok(Value::Array(array_vals));
        }

        Object::Dict(pairs) => {
            let mut obj_map = Map::new();

            for (k, v) in pairs.into_iter() {
                let k_str =
                    String::from_utf8(k.to_vec()).map_err(|e| Error::DictKeyDecodeError(e))?;

                obj_map.insert(k_str, to_json_val(v)?);
            }

            return Ok(Value::Object(obj_map));
        }

        Object::BBytes(bs) => String::from_utf8(bs.to_vec())
            .map_err(|e| Error::StringDecodeError(e))
            .map(|s| Value::String(s)),

        Object::Number(n) => Ok(Value::Number(
            serde_json::Number::from_f64(*n as f64).unwrap(),
        )),
    }
}
