use bytes::{Buf, BytesMut};
use std::ops::ControlFlow;

use super::authentication::Authentication;
use crate::{
    common::BytesRef, general, protocol::{ProtocolDecode, ProtocolError}
};

macro_rules! decode {
    ($ty:ty,$buf:ident) => {
        match <$ty>::decode($buf)? {
            ControlFlow::Break(ok) => ok,
            ControlFlow::Continue(read) => return Ok(ControlFlow::Continue(read)),
        }
    };
}

/// All communication is through a stream of messages.
///
/// 1. The first byte of a message identifies the [message type][BackendMessageFormat]
/// 2. The next four bytes specify the length of the rest of the message
///
/// (this length count includes itself, but not the message-type byte).
/// The remaining contents of the message are determined by the message type.
///
/// <https://www.postgresql.org/docs/current/protocol-overview.html#PROTOCOL-MESSAGE-CONCEPTS>
#[derive(Debug)]
pub enum BackendMessage {
    Authentication(Authentication),
    BackendKeyData(BackendKeyData),
}

impl ProtocolDecode for BackendMessage {
    fn decode(buf: &mut BytesMut) -> Result<ControlFlow<Self,usize>, ProtocolError> {
        // format + len
        const PREFIX: usize = 1 + 4;

        let Some(mut header) = buf.get(..PREFIX) else {
            return Ok(ControlFlow::Continue(PREFIX));
        };

        // The first byte of a message identifies the message type
        let format = header.get_u8();

        let message = match format {
            Authentication::FORMAT => Self::Authentication(decode!(Authentication,buf)),
            BackendKeyData::FORMAT => Self::BackendKeyData(decode!(BackendKeyData,buf)),
            f => return Err(ProtocolError::new(general!(
                "unsupported backend message {:?}",
                BytesRef(&[f])
            ))),
        };

        Ok(ControlFlow::Break(message))
    }
}

//
// NOTE: Backend Messages
//

/// Identifies the message as cancellation key data.
///
/// The frontend must save these values if it wishes to be able to issue CancelRequest messages later.
#[derive(Debug)]
pub struct BackendKeyData {
    /// The process ID of this backend.
    pub process_id: i32,
    /// The secret key of this backend.
    pub secret_key: i32,
}

impl BackendKeyData {
    pub const FORMAT: u8 = b'K';
}

impl ProtocolDecode for BackendKeyData {
    fn decode(buf: &mut BytesMut) -> Result<ControlFlow<Self,usize>, ProtocolError> {
        // format + len + pid + secret
        const PREFIX: usize = 1 + 4 + 4 + 4;

        let Some(mut header) = buf.get(..PREFIX) else {
            return Ok(ControlFlow::Continue(PREFIX));
        };

        let format = header.get_u8();
        if format != Self::FORMAT {
            return Err(ProtocolError::new(general!(
                "expected BackendKeyData ({:?}), found {:?}",
                BytesRef(&[Self::FORMAT]), BytesRef(&[format]),
            )));
        }

        buf.advance(1 + 4);

        Ok(ControlFlow::Break(Self {
            process_id: buf.get_i32(),
            secret_key: buf.get_i32(),
        }))
    }
}

/// Identifies the message as cancellation key data.
///
/// The frontend must save these values if it wishes to be able to issue CancelRequest messages later.
#[derive(Debug)]
pub struct ParameterStatus {
    /// The name of the run-time parameter being reported.
    pub name: String,
    /// The current value of the parameter.
    pub value: String
}

impl ParameterStatus {
    pub const FORMAT: u8 = b'S';
}

impl ProtocolDecode for ParameterStatus {
    fn decode(buf: &mut BytesMut) -> Result<ControlFlow<Self,usize>, ProtocolError> {
        // format + len
        const PREFIX: usize = 1 + 4;

        let Some(mut header) = buf.get(..PREFIX) else {
            return Ok(ControlFlow::Continue(5));
        };

        let format = header.get_u8();
        if format != Self::FORMAT {
            return Err(ProtocolError::new(general!(
                "expected BackendKeyData ({:?}), found {:?}",
                BytesRef(&[Self::FORMAT]), BytesRef(&[format]),
            )));
        }

        todo!()
    }
}

