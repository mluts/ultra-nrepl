pub mod json;

use std::io::{BufRead, Error as IOErr, Read};

#[derive(Debug)]
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
    ReadError(String),
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

        self.rdr
            .read_until(':' as u8, &mut len_buf)
            .map_err(|e| Error::IOError(e))?;

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

        self.rdr
            .read_exact(vec.as_mut_slice())
            .map_err(|e| Error::IOError(e))?;

        if vec.len() == len as usize {
            Ok(vec)
        } else {
            Err(Error::ReadError(format!(
                "Failed to read string with length: {}",
                len
            )))
        }
    }

    fn read_int(&mut self) -> Result<i64, Error> {
        let mut int_buf: Vec<u8> = vec![];
        self.rdr
            .read_until('e' as u8, &mut int_buf)
            .map_err(|e| Error::IOError(e))?;

        let int_str = String::from_utf8(int_buf)
            .map_err(|e| Error::MalformedInput(format!("Int utf8 error: {}", e)))?;

        let n = int_str
            .parse::<i64>()
            .map_err(|e| Error::MalformedInput(format!("Int parse error: {}", e)))?;

        Ok(n)
    }

    fn decode_token(&mut self) -> Result<Token, Error> {
        let mut buf: [u8; 1] = [0; 1];

        self.rdr
            .read_exact(&mut buf)
            .map_err(|e| Error::IOError(e))?;

        match buf[0] as char {
            'l' => Ok(Token::List),
            'i' => Ok(Token::Int),
            'd' => Ok(Token::Dict),
            'e' => Ok(Token::End),
            v @ '0'..='9' => self.read_length(vec![v as u8]).map(|l| Token::Length(l)),
            c => Err(Error::UnexpectedToken(c)),
        }
    }

    fn push_next_token(&mut self) -> Result<(), Error> {
        let token = self.decode_token()?;
        self.tokens.push(token);
        Ok(())
    }

    fn ensure_token(&mut self) -> Result<(), Error> {
        if self.tokens.is_empty() {
            self.push_next_token()?;
        }
        Ok(())
    }

    fn pop_token(&mut self) -> Result<Token, Error> {
        self.ensure_token()?;
        match self.tokens.pop() {
            Some(token) => Ok(token),
            None => unreachable!(),
        }
    }

    fn peek_token(&mut self) -> Result<&Token, Error> {
        self.ensure_token()?;

        match self.tokens.last() {
            Some(token) => Ok(token),
            None => unreachable!(),
        }
    }

    fn read_dict_pair(&mut self) -> Result<(Vec<u8>, Object), Error> {
        match self.pop_token()? {
            Token::Length(n) => {
                let dict_key = self.read_str(n)?;
                let dict_val = self.read_object()?;
                Ok((dict_key, dict_val))
            }
            t => Err(Error::ExpectedDictKey(t)),
        }
    }

    pub fn read_object(&mut self) -> Result<Object, Error> {
        match self.pop_token()? {
            Token::List => {
                let mut objs_read: Vec<Object> = vec![];

                loop {
                    match self.peek_token()? {
                        Token::End => {
                            self.pop_token()?;
                            return Ok(Object::List(objs_read));
                        }
                        _ => objs_read.push(self.read_object()?),
                    }
                }
            }

            Token::Dict => {
                let mut pairs_read: Vec<(Vec<u8>, Object)> = vec![];

                loop {
                    match self.peek_token()? {
                        Token::End => {
                            self.pop_token()?;
                            return Ok(Object::Dict(pairs_read));
                        }
                        _ => {
                            let pair = self.read_dict_pair()?;
                            pairs_read.push(pair);
                        }
                    }
                }
            }

            Token::Int => {
                let int_num = self.read_int()?;
                Ok(Object::Number(int_num))
            }

            Token::Length(n) => {
                let bytes = self.read_str(n)?;
                Ok(Object::BBytes(bytes))
            }
            t => Err(Error::ExpectedObjectStart(t)),
        }
    }
}
