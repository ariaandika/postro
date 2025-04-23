use std::{fmt, str::Utf8Error, string::FromUtf8Error};

pub enum DecodeError {
    Utf8(Utf8Error),
    FieldNotFound,
    IndexOutOfBound,
    OidMissmatch,
}

impl std::error::Error for DecodeError { }

impl fmt::Display for DecodeError {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl fmt::Debug for DecodeError {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
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

