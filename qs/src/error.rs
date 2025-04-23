use std::{fmt, io, str::Utf8Error};

use crate::{
    decode::DecodeError,
    options::ConfigError,
    postgres::{ErrorResponse, ProtocolError},
};

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// All possible error from qs library.
pub enum Error {
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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Config(e) => write!(f, "Configuration error: {e}"),
            Error::Protocol(e) => write!(f, "{e}"),
            Error::Io(e) => write!(f, "{e}"),
            Error::Database(e) => write!(f, "{e}"),
            Error::UnsupportedAuth => write!(f, "Auth not supported"),
            Error::Decode(e) => write!(f, "{e}"),
            Error::MissmatchDataType => write!(f, "Missmatch datatype"),
            Error::ColumnIndexOutOfBounds => write!(f, "Column index out of bounds"),
            Error::Utf8(e) => write!(f, "{e}"),
        }
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
                $body
            }
        }
    };
}

from!(<Utf8Error>e => Self::Utf8(e));
from!(<ProtocolError>e => Self::Protocol(e));
from!(<DecodeError>e => Self::Decode(e));
from!(<ConfigError>e => Self::Config(e));
from!(<std::io::Error>e => Self::Io(e));
from!(<ErrorResponse>e => Self::Database(e));

