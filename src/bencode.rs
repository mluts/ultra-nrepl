//! Decodes bencode from `BufRead + Read`
//!
//! # Purpose
//!
//! It was written due to absence of options for decoding bencode objects on unknown length.
//! For example from `std::net::TcpStream`

pub mod json;

use std::io::{BufRead, Error as IOErr, Read};

#[derive(Debug, PartialEq)]
pub enum Object {
    List(Vec<Object>),
    Dict(Vec<(Vec<u8>, Object)>),
    BBytes(Vec<u8>),
    Number(i64),
}

#[derive(Debug)]
pub enum Token {
    List,
    Dict,
    Int,
    Length(u32),
    BBytes(Vec<u8>),
    End,
}

#[derive(Debug)]
pub enum Error {
    UnexpectedEOF,
    UnexpectedToken(char),
    ExpectedDictKey(Token),
    ExpectedObjectStart(Token),
    BadLengthStr,
    MalformedInput(String),
    IOError(IOErr),
    StrReadError(u32),
}

impl std::convert::From<IOErr> for Error {
    fn from(err: IOErr) -> Error {
        Error::IOError(err)
    }
}

pub struct Decoder<'buf, R: BufRead + Read> {
    rdr: &'buf mut R,
    tokens: Vec<Token>,
}

impl<'buf, R: BufRead + Read> Decoder<'buf, R> {
    pub fn new(rdr: &'buf mut R) -> Decoder<'buf, R> {
        Decoder {
            rdr,
            tokens: vec![],
        }
    }

    fn read_length(&mut self, prefix: Vec<u8>) -> Result<u32, Error> {
        let mut len_buf: Vec<u8> = prefix;

        self.rdr.read_until(':' as u8, &mut len_buf)?;

        match len_buf.pop().map(|c| c as char) {
            Some(':') => (),
            Some(c) => {
                return Err(Error::MalformedInput(format!(
                    "Expected : for length, but had: {}",
                    c as char
                )))
            }
            None => return Err(Error::UnexpectedEOF),
        }

        let len_str = String::from_utf8(len_buf)
            .map_err(|e| Error::MalformedInput(format!("Length string error: {}", e)))?;
        let size = len_str.parse::<u32>().map_err(|_e| Error::BadLengthStr)?;

        Ok(size)
    }

    fn read_str(&mut self, len: u32) -> Result<Vec<u8>, Error> {
        let mut vec: Vec<u8> = vec![];

        vec.resize(len as usize, 0);

        self.rdr.read_exact(vec.as_mut_slice())?;

        if vec.len() == len as usize {
            Ok(vec)
        } else {
            Err(Error::StrReadError(len))
        }
    }

    fn read_int(&mut self) -> Result<i64, Error> {
        let mut int_buf: Vec<u8> = vec![];

        self.rdr.read_until('e' as u8, &mut int_buf)?;

        match int_buf.pop().map(|v| v as char) {
            Some('e') => (),
            _ => return Err(Error::UnexpectedEOF),
        }

        let int_str = String::from_utf8(int_buf)
            .map_err(|e| Error::MalformedInput(format!("Int utf8 error: {}", e)))?;

        let n = int_str
            .parse::<i64>()
            .map_err(|e| Error::MalformedInput(format!("Int parse error: {}", e)))?;

        Ok(n)
    }

    fn decode_token(&mut self) -> Result<Option<Token>, Error> {
        let mut buf: [u8; 1] = [0; 1];

        match self.rdr.read_exact(&mut buf) {
            Ok(()) => (),
            Err(e) => match e.kind() {
                std::io::ErrorKind::UnexpectedEof => return Ok(None),
                _err_kind => return Err(Error::from(e)),
            },
        }

        let t = match buf[0] as char {
            'l' => Token::List,
            'i' => Token::Int,
            'd' => Token::Dict,
            'e' => Token::End,
            v @ '0'..='9' => self.read_length(vec![v as u8]).map(|l| Token::Length(l))?,
            c => return Err(Error::UnexpectedToken(c)),
        };

        Ok(Some(t))
    }

    fn push_next_token(&mut self) -> Result<(), Error> {
        if let Some(t) = self.decode_token()? {
            self.tokens.push(t)
        }
        Ok(())
    }

    fn ensure_token(&mut self) -> Result<(), Error> {
        if self.tokens.is_empty() {
            self.push_next_token()?;
        }
        Ok(())
    }

    fn pop_token(&mut self) -> Result<Option<Token>, Error> {
        self.ensure_token()?;
        Ok(self.tokens.pop())
    }

    fn peek_token(&mut self) -> Result<Option<&Token>, Error> {
        self.ensure_token()?;
        Ok(self.tokens.last())
    }

    fn read_dict_pair(&mut self) -> Result<(Vec<u8>, Object), Error> {
        match self.pop_token()?.ok_or(Error::UnexpectedEOF)? {
            Token::Length(n) => {
                let dict_key = self.read_str(n)?;
                let dict_val = self.read_object()?.ok_or(Error::UnexpectedEOF)?;
                Ok((dict_key, dict_val))
            }
            other_token => Err(Error::ExpectedDictKey(other_token)),
        }
    }

    pub fn read_object(&mut self) -> Result<Option<Object>, Error> {
        if let Some(t) = self.pop_token()? {
            match t {
                Token::List => {
                    let mut objs_read: Vec<Object> = vec![];

                    loop {
                        match self.peek_token()?.ok_or(Error::UnexpectedEOF)? {
                            Token::End => {
                                self.pop_token()?;
                                return Ok(Some(Object::List(objs_read)));
                            }
                            _ => objs_read.push(self.read_object()?.ok_or(Error::UnexpectedEOF)?),
                        }
                    }
                }

                Token::Dict => {
                    let mut pairs_read: Vec<(Vec<u8>, Object)> = vec![];

                    loop {
                        match self.peek_token()? {
                            Some(Token::End) => {
                                self.pop_token()?.unwrap();
                                return Ok(Some(Object::Dict(pairs_read)));
                            }
                            Some(_else) => {
                                let pair = self.read_dict_pair()?;
                                pairs_read.push(pair);
                            }
                            None => return Err(Error::UnexpectedEOF),
                        }
                    }
                }

                Token::Int => {
                    let int_num = self.read_int()?;
                    Ok(Some(Object::Number(int_num)))
                }

                Token::Length(n) => {
                    let bytes = self.read_str(n)?;
                    Ok(Some(Object::BBytes(bytes)))
                }
                t => Err(Error::ExpectedObjectStart(t)),
            }
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn empty_buffer_test() {
        let mut buf: BufReader<&[u8]> = BufReader::new(&[0; 0]);
        let mut decoder = Decoder::new(&mut buf);
        assert_eq!(None, decoder.read_object().unwrap());
    }

    #[test]
    fn integer_test() {
        let ser = b"i-42e";
        let mut buf: BufReader<&[u8]> = BufReader::new(ser);
        let mut decoder = Decoder::new(&mut buf);
        assert_eq!(Some(Object::Number(-42)), decoder.read_object().unwrap());
    }

    #[test]
    fn integer_panic_test() -> Result<(), String> {
        let ser = b"i-42";
        let mut buf: BufReader<&[u8]> = BufReader::new(ser);
        let mut decoder = Decoder::new(&mut buf);
        match decoder.read_object() {
            Ok(_) => Err(format!(
                "Expected to have UnexpectedEOF when int code was not finished!"
            )),
            Err(_) => Ok(()),
        }
    }
}
