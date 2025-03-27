use bytes::BytesMut;

use crate::protocol::ProtocolEncode;

/// https://www.postgresql.org/docs/current/protocol-overview.html#PROTOCOL-MESSAGE-CONCEPTS
///
/// All communication is through a stream of messages. The first byte of a message
/// identifies the [message type][MessageType],
/// and the next four bytes specify the length of the rest of the message
/// (this length count includes itself, but not the message-type byte).
/// The remaining contents of the message are determined by the message type.
/// For historical reasons, the very first message sent by the client
/// (the startup message) has no initial message-type byte.
pub enum FrontendMessage {
    Startup(messages::Startup)
}

impl ProtocolEncode for FrontendMessage {
    fn write(&self, buf: &mut BytesMut) -> Result<(), crate::protocol::ProtocolError> {
        match self {
            FrontendMessage::Startup(s) => s.write(buf),
        }
    }
}

pub mod messages {
    use bytes::{BufMut, Bytes, BytesMut};

    use crate::{err, protocol::{ProtocolEncode, ProtocolError}};

    use super::FrontendMessage;

    pub struct Startup {
        pub user: Bytes,
    }

    impl ProtocolEncode for Startup {
        fn write(&self, buf: &mut BytesMut) -> Result<(), ProtocolError> {
            let offset = buf.len();

            // https://www.postgresql.org/docs/current/protocol-message-formats.html

            // Int32
            // Length of message contents in bytes, including self.
            // len + ver + conten
            buf.put_u32(0);

            // Int32(196608)
            // The protocol version number.
            // The most significant 16 bits are the major version number (3 for the protocol described here).
            // The least significant 16 bits are the minor version number (0 for the protocol described here).
            buf.put_i32(196608);

            // The protocol version number is followed by one or more pairs of parameter name and value strings.
            buf.put_slice(b"user");
            buf.put_u8(0);
            buf.put_slice(&self.user);
            buf.put_u8(0);

            // A zero byte is required as a terminator
            // after the last name/value pair.
            buf.put_u8(0);

            let size = buf.len() - offset;
            let Ok(size) = i32::try_from(size) else {
                return Err(ProtocolError::new(err!("message size out of range for protocol: {size}")));
            };

            buf[offset..(offset + 4)].copy_from_slice(&size.to_be_bytes());

            Ok(())
        }
    }

    impl From<Startup> for FrontendMessage {
        fn from(value: Startup) -> Self {
            FrontendMessage::Startup(value)
        }
    }
}

