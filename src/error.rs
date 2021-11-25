use std::fmt::{Debug, Display, Formatter};

/// An encoding/decoding error.
#[derive(Debug)]
pub enum Error {
    /// There was a read/write error.
    Io(std::io::Error),

    /// You tried to draw from an empty iterator.
    IteratorEmpty,
}

impl From<std::io::Error> for Error {
    #[inline]
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}
