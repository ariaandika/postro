use bytes::{Buf, BytesMut};
use std::ops::ControlFlow;

use super::authentication::Authentication;
use crate::{
    common::{general, BytesRef},
    protocol::{ProtocolDecode, ProtocolError},
};

macro_rules! decode {
    ($ty:ty,$buf:ident) => {
        match <$ty>::decode($buf)? {
            ControlFlow::Break(ok) => ok,
            ControlFlow::Continue(read) => return Ok(ControlFlow::Continue(read)),
        }
    };
}

macro_rules! read_format {
    (@ $buf:ident,$id:ident,$method:ident) => {{
        // format + len
        const FORMAT: usize = 1;
        const PREFIX: usize = FORMAT + 4;

        let Some(mut header) = $buf.get(..PREFIX) else {
            return Ok(ControlFlow::Continue(PREFIX));
        };

        let format = header.get_u8();
        if format != $id::FORMAT {
            return Err(ProtocolError::new(general!(
                "expected {} ({:?}), found {:?}",
                stringify!($id), BytesRef(&[$id::FORMAT]), BytesRef(&[format]),
            )));
        }

        let body_len = header.get_i32() as usize;

        if $buf.get(PREFIX..FORMAT + body_len).is_none() {
            return Ok(ControlFlow::Continue(FORMAT + body_len));
        }

        $buf.advance(PREFIX);
        $buf.$method(body_len - 4)
    }};
    ($buf:ident,$id:ident) => {{
        read_format!(@ $buf,$id,split_to)
    }};
    ($buf:ident,$id:ident,$method:ident) => {{
        read_format!(@ $buf,$id,$method)
    }};
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
    ErrorResponse(ErrorResponse),
    ParameterStatus(ParameterStatus),
    ReadyForQuery(ReadyForQuery),
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
            ErrorResponse::FORMAT => Self::ErrorResponse(decode!(ErrorResponse,buf)),
            ParameterStatus::FORMAT => Self::ParameterStatus(decode!(ParameterStatus,buf)),
            ReadyForQuery::FORMAT => Self::ReadyForQuery(decode!(ReadyForQuery,buf)),
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
        let mut body = read_format!(buf,BackendKeyData);
        Ok(ControlFlow::Break(Self {
            process_id: body.get_i32(),
            secret_key: body.get_i32(),
        }))
    }
}

/// Identifies the message as a run-time parameter status report
#[derive(Debug)]
pub struct ParameterStatus {
    /// The name of the run-time parameter being reported
    pub name: String,
    /// The current value of the parameter
    pub value: String
}

impl ParameterStatus {
    pub const FORMAT: u8 = b'S';
}

macro_rules! nul_string {
    (@ $msg:ident,$advance:stmt) => {{
        let end = match $msg.iter().position(|e|matches!(e,b'\0')) {
            Some(ok) => ok,
            None => return Err(ProtocolError::new(general!(
                        "no nul termination in ParameterStatus",
            )))
        };
        match String::from_utf8($msg.split_to(end).into()) {
            Ok(ok) => {
                $advance
                ok
            },
            Err(err) => return Err(ProtocolError::new(general!(
                "non UTF-8 string in ParameterStatus: {err}",
            ))),
        }
    }};
    ($msg:ident) => {{
        nul_string!(@ $msg,$msg.advance(1))
    }};
    ($msg:ident,noadvance) => {{
        nul_string!(@ $msg,())
    }};
}

impl ProtocolDecode for ParameterStatus {
    fn decode(buf: &mut BytesMut) -> Result<ControlFlow<Self,usize>, ProtocolError> {
        let mut body = read_format!(buf,ParameterStatus);
        let name = nul_string!(body);
        let value = nul_string!(body);
        Ok(ControlFlow::Break(Self { name, value, }))
    }
}

#[derive(Debug)]
pub struct ReadyForQuery;

impl ReadyForQuery {
    pub const FORMAT: u8 = b'Z';
}

impl ProtocolDecode for ReadyForQuery {
    fn decode(buf: &mut BytesMut) -> Result<ControlFlow<Self,usize>, ProtocolError> {
        read_format!(buf,ReadyForQuery,advance);
        Ok(ControlFlow::Break(Self))
    }
}

/// Identifies the message as an error
///
/// The message body consists of one or more identified fields, followed by a zero byte as a terminator. Fields can appear in any order. For each field there is the following:
///
/// Byte1 A code identifying the field type; if zero, this is the message terminator and no string follows. The presently defined field types are listed in Section 53.8. Since more field types might be added in future, frontends should silently ignore fields of unrecognized type.
///
/// String The field value.
///
/// TODO: translate the error response
#[derive(Debug, thiserror::Error)]
#[error("{body:?}")]
pub struct ErrorResponse {
    pub body: std::collections::HashMap<u8,String>,
}

pub const FORMAT: u8 = b'M';

impl ErrorResponse {
    pub const FORMAT: u8 = b'E';
}

impl ProtocolDecode for ErrorResponse {
    fn decode(buf: &mut BytesMut) -> Result<ControlFlow<Self,usize>, ProtocolError> {
        let mut bytes = read_format!(buf,ErrorResponse);
        let mut body = std::collections::HashMap::new();

        loop {
            let f = bytes.get_u8();
            if f == b'\0' {
                break
            }
            let msg = nul_string!(bytes);
            body.insert(f, msg);

        }

        Ok(ControlFlow::Break(Self { body }))
    }
}

