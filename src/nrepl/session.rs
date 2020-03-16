use crate::nrepl;

pub enum Error {
    NreplError(nrepl::Error),
}

impl From<nrepl::Error> for Error {
    fn from(e: nrepl::Error) -> Self {
        Self::NreplError(e)
    }
}
