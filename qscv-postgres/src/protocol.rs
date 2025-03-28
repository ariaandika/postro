use std::ops::ControlFlow;
use bytes::{Bytes, BytesMut};

use crate::common::BoxError;

/// buffered protocol encoding
///
/// the message should write buffer into provided `buf`
pub trait ProtocolEncode {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ProtocolError>;
}

/// buffered protocol decoding
///
/// If decode return [`ControlFlow::Continue`],
/// more read is performed until expected total length in `Continue` is reached.
/// This process repeated until [`ControlFlow::Break`]
/// is returned with the complete message
///
/// If decode returns [`ControlFlow::Continue`], the given
/// `Bytes` should not be owned somewhere, in other word, it should be dropped,
/// so the buffer owner can reclaim the `Bytes` back and read more.
///
/// If decode returns [`ControlFlow::Break`], the given
/// `Bytes` should be split to read bytes, not cloned,
/// so the buffer owner can reclaim it back for the next encoding.
pub trait ProtocolDecode: Sized {
    fn decode(buf: &mut Bytes) -> Result<ControlFlow<Self,usize>, ProtocolError>;
}

/// an error when translating buffer
#[derive(Debug, thiserror::Error)]
#[error("ProtocolError: {source}")]
pub struct ProtocolError {
    source: BoxError,
}

impl From<BoxError> for ProtocolError {
    fn from(value: BoxError) -> Self {
        Self { source: value }
    }
}

impl ProtocolError {
    /// create new [`ProtocolError`]
    pub fn new(source: impl Into<BoxError>) -> Self {
        Self { source: source.into() }
    }
}


