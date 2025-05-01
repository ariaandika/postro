use std::{fmt, io, str::Utf8Error};

use crate::{
    connection::ConfigError,
    postgres::{ErrorResponse, ProtocolError},
    row::DecodeError,
};

/// A specialized [`Result`] type for qs operation.
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Error {
    bt: std::backtrace::Backtrace,
    kind: ErrorKind,
}

/// All possible error from qs library.
pub enum ErrorKind {
    Config(ConfigError),
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
        match &self.kind {
            ErrorKind::Config(e) => write!(f, "Configuration error: {e}"),
            ErrorKind::Protocol(e) => write!(f, "{e}"),
            ErrorKind::Io(e) => write!(f, "{e}"),
            ErrorKind::Database(e) => write!(f, "{e}"),
            ErrorKind::UnsupportedAuth => write!(f, "Auth not supported"),
            ErrorKind::Decode(e) => write!(f, "{e}"),
            ErrorKind::MissmatchDataType => write!(f, "Missmatch datatype"),
            ErrorKind::ColumnIndexOutOfBounds => write!(f, "Column index out of bounds"),
            ErrorKind::Utf8(e) => write!(f, "{e}"),
        }?;

        if let std::backtrace::BacktraceStatus::Captured = self.bt.status() {
            let mut backtrace = self.bt.to_string();
            write!(f, "\n\n")?;
            if backtrace.starts_with("stack backtrace:") {
                // Capitalize to match "Caused by:"
                backtrace.replace_range(0..1, "S");
            } else {
                // "stack backtrace:" prefix was removed in
                // https://github.com/rust-lang/backtrace-rs/pull/286
                writeln!(f, "Stack backtrace:")?;
            }
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

macro_rules! from {
    (<$ty:ty>$pat:pat => $body:expr) => {
        impl From<$ty> for Error {
            fn from($pat: $ty) -> Self {
                let bt = std::backtrace::Backtrace::capture();
                Self { bt, kind: $body }
            }
        }
    };
}

from!(<ErrorKind>e => e);
from!(<Utf8Error>e => ErrorKind::Utf8(e));
from!(<ProtocolError>e => ErrorKind::Protocol(e));
from!(<DecodeError>e => ErrorKind::Decode(e));
from!(<ConfigError>e => ErrorKind::Config(e));
from!(<std::io::Error>e => ErrorKind::Io(e));
from!(<ErrorResponse>e => ErrorKind::Database(e));

