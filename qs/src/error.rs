use std::{fmt, io, str::Utf8Error};

use crate::{
    connection::ParseError,
    postgres::{ErrorResponse, ProtocolError},
    row::DecodeError,
};

/// A specialized [`Result`] type for qs operation.
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Error {
    context: String,
    backtrace: std::backtrace::Backtrace,
    kind: ErrorKind,
}

/// All possible error from qs library.
pub enum ErrorKind {
    Config(ParseError),
    Protocol(ProtocolError),
    Io(io::Error),
    Database(ErrorResponse),
    UnsupportedAuth,
    Decode(DecodeError),
    MissmatchDataType,
    ColumnIndexOutOfBounds,
    Utf8(std::str::Utf8Error),
}

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
            Self::Config(e) => write!(f, "Configuration error: {e}"),
            Self::Protocol(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "{e}"),
            Self::Database(e) => write!(f, "{e}"),
            Self::UnsupportedAuth => write!(f, "Auth not supported"),
            Self::Decode(e) => write!(f, "{e}"),
            Self::MissmatchDataType => write!(f, "Missmatch datatype"),
            Self::ColumnIndexOutOfBounds => write!(f, "Column index out of bounds"),
            Self::Utf8(e) => write!(f, "{e}"),
        }
    }
}

impl fmt::Debug for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{self}\"")
    }
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
from!(<Utf8Error>e => ErrorKind::Utf8(e));
from!(<ProtocolError>e => ErrorKind::Protocol(e));
from!(<DecodeError>e => ErrorKind::Decode(e));
from!(<ParseError>e => ErrorKind::Config(e));
from!(<std::io::Error>e => ErrorKind::Io(e));
from!(<ErrorResponse>e => ErrorKind::Database(e));

