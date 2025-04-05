//! Protocol error
use crate::common::{general, BoxError, BytesRef};

mod database;

pub use database::DatabaseError;

macro_rules! protocol_err {
    (%$str:literal) => {
        crate::message::error::ProtocolError::new($str)
    };
    ($($tt:tt)*) => {
        crate::message::error::ProtocolError::new(format!($($tt)*))
    };
}

pub(crate) use protocol_err;

/// An error when translating buffer from postgres
#[derive(Debug, thiserror::Error)]
#[error("backend protocol error: {source}")]
pub struct ProtocolError {
    source: BoxError,
}

impl ProtocolError {
    /// create new [`ProtocolError`]
    pub fn new(source: impl Into<BoxError>) -> Self {
        Self { source: source.into() }
    }

    pub fn no_nul_string() -> ProtocolError {
        Self { source: general!(%"no nul found in string").into(), }
    }

    pub fn non_utf8(err: impl std::fmt::Display) -> ProtocolError {
        Self { source: general!("non UTF-8 string: {err}").into(), }
    }

    pub fn unexpected(expect: &str, expecttype: u8, found: u8) -> ProtocolError {
        Self {
            source: general!(
                "expected {expect}({:?}) found ({:?})",
                BytesRef(&[expecttype]), BytesRef(&[found]),
            ).into(),
        }
    }

    pub fn unknown(msgtype: u8) -> ProtocolError {
        Self {
            source: general!("unknown message type: {:?}", BytesRef(&[msgtype])).into(),
        }
    }

    pub fn unknown_auth(auth_method: i32) -> ProtocolError {
        Self {
            source: general!("unknown authentication method: ({auth_method})").into(),
        }
    }
}

