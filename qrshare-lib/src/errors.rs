use std::{fmt, io, path::PathBuf};

use actix_web::{body::BoxBody, error::ResponseError, HttpResponse};
use http::status::StatusCode;

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
    /// A [`Mutex`] or [`RwLock`] has been poisoned
    ///
    /// [`Mutex`]: std::sync::Mutex
    /// [`RwLock`]: std::sync::RwLock
    PoisonSync,
    /// An error from [`hyper`]
    Hyper(hyper::Error),
    /// An error from constructing a response.
    Http(http::Error),
    /// An HTTP status code with a body
    HttpResponse(http::StatusCode, String),
    /// Unable to retrieve an outside-facing IPv4 address.
    NoGlobalIpv4,
    /// Cannot parse string into URI
    Uri(String),
    /// An error from [`qrcode`]
    Qr(qrcode::types::QrError),
    /// An error from [`image`]
    Img(image::ImageError),
}

impl From<http::StatusCode> for Error {
    fn from(code: http::StatusCode) -> Self {
        (code, code.to_string()).into()
    }
}

impl<'b, S: AsRef<str> + ?Sized> From<(http::StatusCode, &'b S)> for Error {
    fn from((code, body): (http::StatusCode, &'b S)) -> Self {
        (code, body.to_owned()).into()
    }
}

impl From<(http::StatusCode, String)> for Error {
    fn from((code, body): (http::StatusCode, String)) -> Self {
        Self::HttpResponse(code, body)
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        Self::PoisonSync
    }
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
            Self::NoFiles => write!(f, "No files supplied"),
            Self::InvalidFile(p) => {
                write!(f, "Invalid file at {}", p.display())
            }
            Self::IO(e) => write!(f, "Error from std::io: {}", e),
            Self::JoinPanic => write!(f, "Cannot join task"),
            Self::JoinCancel => write!(f, "Task canceled"),
            Self::PoisonSync => write!(f, "Lock poisoned"),
            Self::NoFifo(p) => write!(f, "FIFO file at {}", p.display()),
            Self::NoGlobalIpv4 => write!(f, "No outside-facing IPv4 address"),
            Self::Uri(s) => write!(f, "Cannot parse as URI: {}", s),
            Self::ArgConflict => write!(f, "Conflicting arguments found"),
            // error objects from external crates
            Self::Hyper(e) => write!(f, "[hyper]: {}", e),
            Self::Http(e) => write!(f, "[http]: {}", e),
            Self::Qr(e) => write!(f, "[qrcode]: {}", e),
            Self::Img(e) => write!(f, "[image]: {}", e),
            Self::HttpResponse(code, body) => write!(f, "({}) {}", code, body),
        }
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::HttpResponse(code, _) => *code,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let mut builder = HttpResponse::build(self.status_code());
        match self {
            Self::HttpResponse(_, body) => builder.body(body.to_owned()),
            _ => builder.body(self.to_string()),
        }
    }
}
