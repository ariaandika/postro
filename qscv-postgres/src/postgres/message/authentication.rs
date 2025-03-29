use bytes::Buf;
use std::ops::ControlFlow;

use crate::{
    general,
    protocol::{ProtocolDecode, ProtocolError},
};

/// <https://www.postgresql.org/docs/current/protocol-message-formats.html#PROTOCOL-MESSAGE-FORMATS>
#[derive(Debug)]
pub enum Authentication {
    /// Int32(8) Length of message contents in bytes, including self.
    /// Int32(0) Specifies that the authentication was successful.
    Ok,
    /// Int32(8) Length of message contents in bytes, including self.
    /// Int32(2) Specifies that Kerberos V5 authentication is required.
    KerberosV5,
    /// Int32(8) Length of message contents in bytes, including self.
    /// Int32(3) Specifies that a clear-text password is required.
    CleartextPassword,
    /// Int32(12) Length of message contents in bytes, including self.
    /// Int32(5) Specifies that an MD5-encrypted password is required.
    /// Byte4 The salt to use when encrypting the password.
    MD5Password {
        salt: u32
    },
    /// Int32(8) Length of message contents in bytes, including self.
    /// Int32(7) Specifies that GSSAPI authentication is required.
    GSS,
    /// Int32(8) Length of message contents in bytes, including self.
    /// Int32(9) Specifies that SSPI authentication is required.
    SSPI,
    /// Int32 Length of message contents in bytes, including self.
    /// Int32(10) Specifies that SASL authentication is required.
    ///   The message body is a list of SASL authentication mechanisms,
    ///   in the server's order of preference. A zero byte is required
    ///   as terminator after the last authentication mechanism name.
    ///   For each mechanism, there is the following:
    /// String Name of a SASL authentication mechanism.
    /// TODO: SASL not yet supported
    /// there are more protocol for SASL control flow
    SASL,
}

impl Authentication {
    pub const FORMAT: u8 = b'R';
}

impl ProtocolDecode for Authentication {
    fn decode(buf: &mut bytes::Bytes) -> Result<ControlFlow<Self,usize>, ProtocolError> {
        let Some(mut header) = buf.get(..5) else {
            return Ok(ControlFlow::Continue(5));
        };

        let format = header.get_u8();
        if format != Self::FORMAT {
            return Err(ProtocolError::new(general!(
                "expected Authentication format ({:x}), found {format:x}",
                Self::FORMAT
            )));
        }

        let len = header.get_i32() as _;

        let body = match buf.get(..len) {
            Some(mut body) => body.get_i32(),
            None => return Ok(ControlFlow::Continue(len)),
        };

        // format + length + body
        buf.advance(1 + 4 + len);

        let auth = match body {
            0 => Authentication::Ok,
            2 => Authentication::KerberosV5,
            3 => Authentication::CleartextPassword,
            5 => {
                let salt = match buf.get(..4) {
                    Some(mut salt) => salt.get_u32(),
                    _ => return Ok(ControlFlow::Continue(4)),
                };
                buf.advance(4);
                Authentication::MD5Password { salt, }
            }
            7 => Authentication::GSS,
            9 => Authentication::SSPI,
            10 => Authentication::SASL,
            f => return Err(ProtocolError::new(general!(
                "unknown authentication methods ({f:x})"
            ))),
        };

        Ok(ControlFlow::Break(auth))
    }
}


