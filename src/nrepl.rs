use bendy::decoding::{Decoder as BDecoder, DictDecoder as BDictDecoder};
use bendy::encoding::{Error as BError, SingleItemEncoder, ToBencode};
use std::fmt;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub enum Error {
    ConnectionLost,
    BencodeEncodeError(BError),
    BencodeDecodeError,
    IOError(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Nrepl Error: {}",
            match self {
                Error::ConnectionLost => format!("Connection lost!"),
                Error::BencodeDecodeError => format!("Bencode decode error"),
                Error::IOError(io_err) => format!("IO Error: {}", io_err),
                Error::BencodeEncodeError(berr) => format!("Bencode encode error: {}", berr),
            }
        )
    }
}

pub struct NreplStream {
    tcp: TcpStream,
}

pub struct Op {
    name: String,
    args: Vec<(String, String)>,
}

impl Op {
    fn new(name: String, args: Vec<(String, String)>) -> Op {
        Op { name, args }
    }
}

struct Resp {
    id: String,
    session: String
}

impl Resp {
    fn decode_resp(dict: BDictDecoder) -> Result<Resp, Error> {
        Err(Error::BencodeDecodeError)
    }
}

impl ToBencode for Op {
    const MAX_DEPTH: usize = 3;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), BError> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"op", &self.name)?;

            for (argname, argval) in self.args.iter() {
                e.emit_pair(&argname.clone().into_bytes(), argval)?
            }

            Ok(())
        })?;
        Ok(())
    }
}

impl NreplStream {
    pub fn connect_timeout(addr: &SocketAddr) -> Result<NreplStream, Error> {
        TcpStream::connect_timeout(addr, Duration::new(3, 0))
            .map(|s| NreplStream { tcp: s })
            .map_err(|e| Error::IOError(e))
    }

    fn send_op(&self, op: &Op) -> Result<(), Error> {
        let mut bw = BufWriter::new(&self.tcp);
        let bencode = op.to_bencode().map_err(|e| Error::BencodeEncodeError(e))?;
        bw.write(&bencode).map_err(|e| Error::IOError(e))?;
        Ok(())
    }

    fn read_resp(&self) -> Result<Resp, Error> {
        // let br = BufReader::new(&self.tcp);
        // let decoder = BDecoder::new(br);
        
        Err(Error::BencodeDecodeError)
    }
}
