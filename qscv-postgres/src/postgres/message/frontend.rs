use bytes::{BufMut, BytesMut};

use crate::{common::general, protocol::{ProtocolEncode, ProtocolError}};


#[derive(Debug)]
pub struct PasswordMessage<'a> {
    pub len: i32,
    pub password: &'a str,
}

impl PasswordMessage<'_> {
    pub const FORMAT: u8 = b'p';
}

impl ProtocolEncode for PasswordMessage<'_> {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ProtocolError> {
        let offset = buf.len();

        // Byte1('p') Identifies the message as a password response
        buf.put_u8(Self::FORMAT);

        // Int32 Length of message contents in bytes, including self.
        // reserve 4 bytes for length
        buf.put_u32(0);

        // String The password (encrypted, if requested)
        buf.put(self.password.as_bytes());
        buf.put_u8(b'\0');

        // write the length afterwards
        let size = buf[1..].len() - offset;
        let Ok(size) = i32::try_from(size) else {
            return Err(ProtocolError::new(general!("message size out of range for protocol: {size}")));
        };

        buf[1..][offset..(offset + 4)].copy_from_slice(&size.to_be_bytes());

        Ok(())
    }
}


