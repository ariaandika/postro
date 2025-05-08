//! `postro` error types.
use std::{backtrace::Backtrace, fmt, io, str::Utf8Error};

use crate::{
    connection::ParseError,
    fetch::EmptyQueryError,
    phase::UnsupportedAuth,
    postgres::{ErrorResponse, ProtocolError},
    row::{DecodeError, RowNotFound},
};

/// A specialized [`Result`] type for `postro` operation.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// All possible error from `postro` library.
pub struct Error {
    context: String,
    backtrace: Backtrace,
    kind: ErrorKind,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

/// All possible error kind from `postro` library.
pub enum ErrorKind {
    Config(ParseError),
    Protocol(ProtocolError),
    Io(io::Error),
    Database(ErrorResponse),
    Utf8(std::str::Utf8Error),
    RowNotFound(RowNotFound),
    EmptyQuery(EmptyQueryError),
    UnsupportedAuth(UnsupportedAuth),
    Decode(DecodeError),
}

macro_rules! from {
    (<$ty:ty>$pat:pat => $body:expr) => {
        impl From<$ty> for Error {
            fn from($pat: $ty) -> Self {
                let backtrace = std::backtrace::Backtrace::capture();
                Self { context: String::new(), backtrace, kind: $body }
            }
        }
    };
}

from!(<ErrorKind>e => e);
from!(<ParseError>e => ErrorKind::Config(e));
from!(<ProtocolError>e => ErrorKind::Protocol(e));
from!(<std::io::Error>e => ErrorKind::Io(e));
from!(<ErrorResponse>e => ErrorKind::Database(e));
from!(<Utf8Error>e => ErrorKind::Utf8(e));
from!(<RowNotFound>e => ErrorKind::RowNotFound(e));
from!(<EmptyQueryError>e => ErrorKind::EmptyQuery(e));
from!(<UnsupportedAuth>e => ErrorKind::UnsupportedAuth(e));

from!(<DecodeError>e => ErrorKind::Decode(e));

impl std::error::Error for Error { }

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.context.is_empty() {
            write!(f, "{}: ", self.context)?;
        }

        fmt::Display::fmt(&self.kind, f)?;

        if let std::backtrace::BacktraceStatus::Captured = self.backtrace.status() {
            let mut backtrace = self.backtrace.to_string();
            write!(f, "\n\n")?;
            writeln!(f, "Stack backtrace:")?;
            backtrace.truncate(backtrace.trim_end().len());
            write!(f, "{}", backtrace)?;
        }

        Ok(())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{self}\"")
    }
}

impl std::error::Error for ErrorKind { }

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(e) => e.fmt(f),
            Self::Protocol(e) => e.fmt(f),
            Self::Io(e) => e.fmt(f),
            Self::Database(e) => e.fmt(f),
            Self::UnsupportedAuth(e) => e.fmt(f),
            Self::RowNotFound(e) => e.fmt(f),
            Self::EmptyQuery(e) => e.fmt(f),
            Self::Decode(e) => e.fmt(f),
            Self::Utf8(e) => e.fmt(f)
        }
    }
}

impl fmt::Debug for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{self}\"")
    }
}

