use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

/// Represent error that occurred during archive generation.
#[derive(Debug)]
pub enum Error {
    /// Writing error.
    IoError(std::io::Error),
    /// Error during integer conversion.
    ///
    /// There is an upper limit to the size that can be stored in Zip.
    /// You will get this error if you pass too large data.
    IntError(std::num::TryFromIntError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::IoError(error) => Display::fmt(error, f),
            Self::IntError(error) => Display::fmt(error, f),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IoError(error)
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(error: std::num::TryFromIntError) -> Self {
        Error::IntError(error)
    }
}
