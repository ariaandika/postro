//! Protocol error
use crate::common::BoxError;

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
#[error("ProtocolError: {source}")]
pub struct ProtocolError {
    source: BoxError,
}

impl ProtocolError {
    /// create new [`ProtocolError`]
    pub fn new(source: impl Into<BoxError>) -> Self {
        Self { source: source.into() }
    }
}

