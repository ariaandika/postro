use std::ops::ControlFlow;
use bytes::{Bytes, BytesMut};

/// buffered protocol encoding
pub trait ProtocolEncode {
    fn write(&self, buf: &mut BytesMut) -> Result<(), ProtocolError>;
}

/// buffered protocol decoding
///
/// when encoding protocol, [`ProtocolDecode::check`] is first called.
///
/// If its return [`ControlFlow::Continue`] with the expected total length,
/// more read is performed until expected total length is reached.
/// This process repeated until [`ControlFlow::Break`]
/// is returned with the amount of buffered will be consumed.
///
/// Finally, [`ProtocolDecode::consume`] called with expected length
/// buffer to construct the final message.
pub trait ProtocolDecode: Sized {
    fn check(buf: &[u8]) -> Result<ControlFlow<usize,usize>, ProtocolError>;
    fn consume(buf: Bytes) -> Result<Self, ProtocolError>;
}

#[derive(Debug, thiserror::Error)]
#[error("ProtocolError: {source}")]
pub struct ProtocolError {
    source: Box<dyn std::error::Error + Send + Sync + 'static>,
}

impl ProtocolError {
    pub fn new(source: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>) -> Self {
        Self { source: source.into() }
    }
}


