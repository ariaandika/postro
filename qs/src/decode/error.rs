use std::{fmt, str::Utf8Error, string::FromUtf8Error};

/// An error when decoding row value.
pub enum DecodeError {
    /// Postgres return non utf8 string.
    Utf8(Utf8Error),
    /// Field requested not found.
    FieldNotFound,
    /// Index requested is out of bounds.
    IndexOutOfBound,
    /// Oid requested missmatch.
    OidMissmatch,
}

impl std::error::Error for DecodeError { }

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Failed to decode value, ")?;
        match self {
            DecodeError::Utf8(e) => write!(f, "{e}"),
            DecodeError::FieldNotFound => write!(f, "field not found"),
            DecodeError::IndexOutOfBound => write!(f, "index out of bounds"),
            DecodeError::OidMissmatch => write!(f, "data type missmatch"),
        }
    }
}

impl fmt::Debug for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{self}\"")
    }
}

macro_rules! from {
    (<$ty:ty>$pat:pat => $body:expr) => {
        impl From<$ty> for DecodeError {
            fn from($pat: $ty) -> Self {
                $body
            }
        }
    };
}

from!(<Utf8Error>e => Self::Utf8(e));
from!(<FromUtf8Error>e => Self::Utf8(e.utf8_error()));

