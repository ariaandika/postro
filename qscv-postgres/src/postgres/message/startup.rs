use bytes::{BufMut, BytesMut};

use crate::{
    err,
    protocol::{ProtocolEncode, ProtocolError},
};

/// Postgres Startup frontend message
#[derive(Debug)]
pub struct Startup<'a> {
    /// The database user name to connect as. Required; there is no default.
    pub user: &'a str,
    /// The database to connect to. Defaults to the user name.
    pub database: Option<&'a str>,
    /// Used to connect in streaming replication mode, where a small set of
    /// replication commands can be issued instead of SQL statements.
    ///
    /// Value can be true, false, or database, and the default is false.
    pub replication: Option<&'a str>,
}

/// See source code for detailed message
///
/// Fri Mar 28 07:34:09 PM WIB 2025
///
/// <https://www.postgresql.org/docs/current/protocol-message-formats.html#PROTOCOL-MESSAGE-FORMATS-STARTUPMESSAGE>
impl ProtocolEncode for Startup<'_> {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ProtocolError> {
        let offset = buf.len();

        // Int32
        // Length of message contents in bytes, including self.
        // reserve 4 bytes for length
        buf.put_u32(0);

        // Int32(196608)
        // The protocol version number.
        // The most significant 16 bits are the major version number (3 for the protocol described here).
        // The least significant 16 bits are the minor version number (0 for the protocol described here).
        buf.put_i32(196608);

        // The protocol version number is followed by one or more pairs of parameter name and value strings.

        // user: The database user name to connect as. Required; there is no default.

        buf.put_slice(b"user");
        buf.put_u8(0);
        buf.put_slice(&self.user.as_bytes());
        buf.put_u8(0);

        // database: The database to connect to. Defaults to the user name.

        if let Some(db) = self.database {
            buf.put_slice(b"database");
            buf.put_u8(0);
            buf.put_slice(&db.as_bytes());
            buf.put_u8(0);
        }

        // options: Command-line arguments for the backend.
        //    (This is deprecated in favor of setting individual run-time parameters.)
        //    Spaces within this string are considered to separate arguments,
        //    unless escaped with a backslash (\); write \\ to represent a literal backslash.

        // not supported


        // replication: Used to connect in streaming replication mode, where a small set of
        //    replication commands can be issued instead of SQL statements.
        //    Value can be true, false, or database, and the default is false.

        if let Some(repl) = self.replication {
            buf.put_slice(b"replication");
            buf.put_u8(0);
            buf.put_slice(&repl.as_bytes());
            buf.put_u8(0);
        }

        // In addition to the above, other parameters may be listed.
        // Parameter names beginning with _pq_. are reserved for use as protocol extensions,
        // while others are treated as run-time parameters to be set at backend start time.
        // Such settings will be applied during backend start
        // (after parsing the command-line arguments if any) and will act as session defaults.

        // A zero byte is required as a terminator after the last name/value pair.
        buf.put_u8(0);

        // write the length afterwards
        let size = buf.len() - offset;
        let Ok(size) = i32::try_from(size) else {
            return Err(ProtocolError::new(err!("message size out of range for protocol: {size}")));
        };

        buf[offset..(offset + 4)].copy_from_slice(&size.to_be_bytes());

        Ok(())
    }
}

