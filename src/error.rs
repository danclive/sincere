use std::result;
use std::io;
use std::fmt;
use std::error;
use std::sync::mpsc::TryRecvError;
use std::string::FromUtf8Error;
use std::num::ParseIntError;

use soio::channel::SendError;
use soio::tcp::TcpStream;

use serde_json;

use httparse;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    JsonError(serde_json::Error),
    SendSocketError(SendError<TcpStream>),
    ReceiveSocketError(TryRecvError),
    FromUtf8Error(FromUtf8Error),
    HttpParseError(httparse::Error),
    ParseIntError(ParseIntError),
    Error(String),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IoError(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::JsonError(err)
    }
}

impl From<SendError<TcpStream>> for Error {
    fn from(err: SendError<TcpStream>) -> Self {
        Error::SendSocketError(err)
    }
}

impl From<TryRecvError> for Error {
    fn from(err: TryRecvError) -> Self {
        Error::ReceiveSocketError(err)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Self {
        Error::FromUtf8Error(err)
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Self {
        Error::ParseIntError(err)
    }
}

impl From<httparse::Error> for Error {
    fn from(err: httparse::Error) -> Self {
        Error::HttpParseError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::IoError(ref inner) => inner.fmt(fmt),
            Error::JsonError(ref inner) => inner.fmt(fmt),
            Error::SendSocketError(ref inner) => inner.fmt(fmt),
            Error::ReceiveSocketError(ref inner) => inner.fmt(fmt),
            Error::FromUtf8Error(ref inner) => inner.fmt(fmt),
            Error::HttpParseError(ref inner) => inner.fmt(fmt),
            Error::ParseIntError(ref inner) => inner.fmt(fmt),
            Error::Error(ref inner) => inner.fmt(fmt),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::IoError(ref err) => err.description(),
            Error::JsonError(ref err) => err.description(),
            Error::SendSocketError(ref err) => err.description(),
            Error::ReceiveSocketError(ref err) => err.description(),
            Error::FromUtf8Error(ref err) => err.description(),
            Error::HttpParseError(ref err) => err.description(),
            Error::ParseIntError(ref err) => err.description(),
            Error::Error(ref err) => err,
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::IoError(ref err) => Some(err),
            Error::JsonError(ref err) => Some(err),
            Error::SendSocketError(ref err) => Some(err),
            Error::ReceiveSocketError(ref err) => Some(err),
            Error::FromUtf8Error(ref err) => Some(err),
            Error::HttpParseError(ref err) => Some(err),
            Error::ParseIntError(ref err) => Some(err),
            Error::Error(_) => None,
        }
    }
}
