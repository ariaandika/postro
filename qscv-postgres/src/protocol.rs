use bytes::BytesMut;
use std::ops::ControlFlow;

use crate::common::BoxError;

/// buffered protocol encoding
///
/// the message should write buffer into provided `buf`
pub trait ProtocolEncode {
    fn encode(self, buf: &mut BytesMut) -> Result<(), ProtocolError>;
}

/// buffered protocol decoding
///
/// If decode return [`ControlFlow::Continue`],
/// more read is performed until expected *total length* in `Continue` is reached.
/// This process repeated until [`ControlFlow::Break`]
/// is returned with the complete message
///
/// If decode returns [`ControlFlow::Continue`], the given
/// `BytesMut` *should not* be modified in any way, so more read
/// does not shuffle the bytes order
///
/// If decode returns [`ControlFlow::Break`], the given
/// `BytesMut` should be split to the required amount,
/// the leftover bytes used for the next decoder
pub trait ProtocolDecode: Sized {
    fn decode(buf: &mut BytesMut) -> Result<ControlFlow<Self,usize>, ProtocolError>;
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


