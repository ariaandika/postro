use bytes::{Buf, BytesMut};
use std::ops::ControlFlow;

use super::authentication::Authentication;
use crate::{
    general, protocol::{ProtocolDecode, ProtocolError}
};

macro_rules! decode {
    ($ty:ty,$buf:ident) => {
        match <$ty>::decode($buf)? {
            ControlFlow::Break(ok) => ok,
            ControlFlow::Continue(read) => return Ok(ControlFlow::Continue(read)),
        }
    };
}

#[derive(Debug)]
#[repr(u8)]
pub enum BackendMessageFormat {
    /// Identifies the message as an authentication request
    Authentication = b'R',
}

impl BackendMessageFormat {
    /// from postgres first byte format
    pub fn from_u8(id: u8) -> Option<Self> {
        match id {
            b'R' => Some(Self::Authentication),
            _ => None,
        }
    }
}

/// <https://www.postgresql.org/docs/current/protocol-overview.html#PROTOCOL-MESSAGE-CONCEPTS>
///
/// All communication is through a stream of messages.
///
/// 1. The first byte of a message identifies the [message type][BackendMessageFormat]
/// 2. The next four bytes specify the length of the rest of the message
///
/// (this length count includes itself, but not the message-type byte).
/// The remaining contents of the message are determined by the message type.
#[derive(Debug)]
pub enum BackendMessage {
    Authentication(Authentication),
}

impl ProtocolDecode for BackendMessage {
    fn decode(buf: &mut BytesMut) -> Result<ControlFlow<Self,usize>, ProtocolError> {
        let Some(mut header) = buf.get(..5) else {
            return Ok(ControlFlow::Continue(5));
        };

        // The first byte of a message identifies the message type
        let format = header.get_u8();
        let Some(format) = BackendMessageFormat::from_u8(format) else {
            return Err(ProtocolError::new(general!(
                "unsupported backend message {:?}",
                bytes::Bytes::copy_from_slice(&[format])
            )));
        };

        let message = match format {
            BackendMessageFormat::Authentication => Self::Authentication(decode!(Authentication,buf)),
        };

        Ok(ControlFlow::Break(message))
    }
}

