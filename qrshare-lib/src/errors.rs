use std::{fmt, io, path::PathBuf};

#[non_exhaustive]
#[derive(Debug)]
/// Errors for this crate
pub enum Error {
    /// When no files are supplied
    NoFiles, // "Supply at least one file"
    /// When options are in conflict
    ArgConflict,
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
    /// An error from constructing a response.
    Http(http::Error),
    /// Unable to retrieve an outside-facing IPv4 address.
    NoGlobalIpv4,
    /// Cannot parse string into URI
    Uri(String),
    /// An error from [`qrcode`]
    Qr(qrcode::types::QrError),
    /// An error from [`image`]
    Img(image::ImageError),
}

impl From<image::ImageError> for Error {
    fn from(v: image::ImageError) -> Self {
        Self::Img(v)
    }
}

impl From<qrcode::types::QrError> for Error {
    fn from(v: qrcode::types::QrError) -> Self {
        Self::Qr(v)
    }
}

impl From<http::Error> for Error {
    fn from(v: http::Error) -> Self {
        Self::Http(v)
    }
}

impl From<hyper::Error> for Error {
    fn from(v: hyper::Error) -> Self {
        Self::Hyper(v)
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

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // local errors
            Error::NoFiles => write!(f, "No files supplied"),
            Error::InvalidFile(p) => {
                write!(f, "Invalid file at {}", p.display())
            }
            Error::IO(e) => write!(f, "Error from std::io: {}", e),
            Error::JoinPanic => write!(f, "Cannot join task"),
            Error::JoinCancel => write!(f, "Task canceled"),
            Error::NoFifo(p) => write!(f, "FIFO file at {}", p.display()),
            Error::NoGlobalIpv4 => write!(f, "No outside-facing IPv4 address"),
            Error::Uri(s) => write!(f, "Cannot parse as URI: {}", s),
            Error::ArgConflict => write!(f, "Conflicting arguments found"),
            // error objects from external crates
            Error::Hyper(e) => write!(f, "[hyper]: {}", e),
            Error::Http(e) => write!(f, "[http]: {}", e),
            Error::Qr(e) => write!(f, "[qrcode]: {}", e),
            Error::Img(e) => write!(f, "[image]: {}", e),
        }
    }
}
