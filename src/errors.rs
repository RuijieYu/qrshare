use core::fmt;
use std::io;
use std::path::PathBuf;

#[non_exhaustive]
#[derive(Debug)]
/// Errors for this crate
pub enum Error {
    /// When no files are supplied
    NoFiles, // "Supply at least one file"
    /// When a file is invalid (not an existing and readable FIFO or regular
    /// file)
    InvalidFile(PathBuf),
    /// FIFO is currently not supported
    NoFifo(PathBuf),
    /// An io error
    IO(io::ErrorKind),
    /// A file-serving thread has panicked
    JoinPanic,
    /// A file-serving thread has been cancelled
    JoinCancel,
    /// An error from [`hyper`]
    Hyper(hyper::Error),
}

impl From<hyper::Error> for Error {
    fn from(v: hyper::Error) -> Self {
        Self::Hyper(v)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NoFiles => write!(f, "No files supplied"),
            Error::InvalidFile(p) => {
                write!(f, "Invalid file at {}", p.display())
            }
            Error::IO(e) => write!(f, "Error from std::io: {}", e),
            Error::JoinPanic => write!(f, "Cannot join task"),
            Error::JoinCancel => write!(f, "Task canceled"),
            Error::NoFifo(p) => write!(f, "FIFO file at {}", p.display()),
            Error::Hyper(e) => write!(f, "Hyper: {}", e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::IO(e.kind())
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(e: tokio::task::JoinError) -> Self {
        if e.is_panic() {
            Self::JoinPanic
        } else {
            Self::JoinCancel
        }
    }
}
