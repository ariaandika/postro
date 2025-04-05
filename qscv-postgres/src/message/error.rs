use crate::common::BoxError;

/// an error when translating buffer
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

