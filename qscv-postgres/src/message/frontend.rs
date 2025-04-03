use bytes::{BufMut, Bytes, BytesMut};

use crate::{
    common::general,
    protocol::{ProtocolEncode, ProtocolError},
};

/// write the buffer length at the first 4 bytes
///
/// note to exclude the message format when writing postgres message length
fn pg_write_len(mut buf: &mut [u8]) -> Result<(), ProtocolError> {
    let size = buf.len();
    let Ok(size) = i32::try_from(size) else {
        return Err(ProtocolError::new(general!("message size out of range for protocol: {size}")));
    };

    buf.put_i32(size);

    Ok(())
}

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
    fn encode(self, buf: &mut BytesMut) -> Result<(), ProtocolError> {
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
        buf.put_slice(self.user.as_bytes());
        buf.put_u8(0);

        // database: The database to connect to. Defaults to the user name.

        if let Some(db) = self.database {
            buf.put_slice(b"database");
            buf.put_u8(0);
            buf.put_slice(db.as_bytes());
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
            buf.put_slice(repl.as_bytes());
            buf.put_u8(0);
        }

        // In addition to the above, other parameters may be listed.
        // Parameter names beginning with _pq_. are reserved for use as protocol extensions,
        // while others are treated as run-time parameters to be set at backend start time.
        // Such settings will be applied during backend start
        // (after parsing the command-line arguments if any) and will act as session defaults.

        // A zero byte is required as a terminator after the last name/value pair.
        buf.put_u8(0);

        // write the length, Startup has no message format
        pg_write_len(&mut buf[offset..])?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct PasswordMessage<'a> {
    pub len: i32,
    pub password: &'a str,
}

impl PasswordMessage<'_> {
    pub const FORMAT: u8 = b'p';
}

impl ProtocolEncode for PasswordMessage<'_> {
    fn encode(self, buf: &mut BytesMut) -> Result<(), ProtocolError> {
        let offset = buf.len();

        // Byte1('p') Identifies the message as a password response
        buf.put_u8(Self::FORMAT);

        // Int32 Length of message contents in bytes, including self.
        // reserve 4 bytes for length
        buf.put_u32(0);

        // String The password (encrypted, if requested)
        buf.put(self.password.as_bytes());
        buf.put_u8(b'\0');

        // write the length, excluding msg format
        pg_write_len(&mut buf[offset + 1..])?;

        Ok(())
    }
}

/// Identifies the message as a simple query
pub struct Query {
    /// the query string itself
    query: Bytes,
}

impl Query {
    pub fn new(query: impl Into<Bytes>) -> Self {
        Self { query: query.into() }
    }

    pub const FORMAT: u8 = b'Q';
}

impl ProtocolEncode for Query {
    fn encode(self, buf: &mut BytesMut) -> Result<(), ProtocolError> {
        let offset = buf.len();

        // Byte1('Q') Identifies the message as a simple query.
        buf.put_u8(Self::FORMAT);

        // Int32 Length of message contents in bytes, including self.
        // reserve 4 bytes for length
        buf.put_u32(0);

        // String The query string itself
        buf.put(&self.query[..]);
        // C style string
        buf.put_u8(b'\0');

        // write the length, excluding msg format
        pg_write_len(&mut buf[offset + 1..])?;

        Ok(())
    }
}

/// Identifies the message as a Parse command
pub struct Parse<'a,I> {
    /// prepared statement name (an empty string selects the unnamed prepared statement).
    pub name: &'a [u8],
    /// The query string to be parsed.
    pub query: &'a [u8],
    /// The number of parameter data types specified (can be zero).
    ///
    /// Note that this is not an indication of the number of parameters that might appear in the query string,
    /// only the number that the frontend wants to prespecify types for.
    ///
    /// For each parameter, there is the following `data_types`
    pub data_types_len: i16,
    /// Specifies the object ID of the parameter data type.
    ///
    /// Placing a zero here is equivalent to leaving the type unspecified.
    pub data_types: I,//&'a [i32],
}

impl<I> Parse<'_,I> {
    pub const FORMAT: u8 = b'P';
}

impl<I> ProtocolEncode for Parse<'_,I>
where
    I: IntoIterator<Item = i32>
{
    fn encode(self, buf: &mut BytesMut) -> Result<(), ProtocolError> {
        let offset = buf.len();

        // message format
        buf.put_u8(Self::FORMAT);

        // Length of message contents
        buf.put_u32(0);

        // prepared statement name
        buf.put(self.name);
        buf.put_u8(b'\0');

        // The query string to be parsed.
        buf.put(self.query);
        buf.put_u8(b'\0');

        // The number of parameter data types specified (can be zero).
        //
        // Note that this is not an indication of the number of parameters that might appear in the query string,
        // only the number that the frontend wants to prespecify types for.
        //
        // For each parameter, there is the following `data_types`
        buf.put_i16(self.data_types_len);

        // Specifies the object ID of the parameter data type.
        //
        // Placing a zero here is equivalent to leaving the type unspecified.
        for dt in self.data_types {
            buf.put_i32(dt);
        }

        // write the length, excluding msg format
        pg_write_len(&mut buf[offset + 1..])?;

        Ok(())
    }
}

/// Identifies the message as a Sync command
pub struct Sync;

impl Sync {
    pub const FORMAT: u8 = b'S';
}

impl ProtocolEncode for Sync {
    fn encode(self, buf: &mut BytesMut) -> Result<(), ProtocolError> {
        buf.put_u8(Self::FORMAT);
        buf.put_i32(4);
        Ok(())
    }
}


/// Identifies the message as a Bind command
pub struct Bind<'a,I,P,R> {
    /// The name of the destination portal (an empty string selects the unnamed portal).
    pub portal_name: &'a [u8],
    /// The name of the source prepared statement (an empty string selects the unnamed prepared statement).
    pub prepare_name: &'a [u8],

    /// The number of parameter format codes that follow (denoted C below).
    /// This can be zero to indicate that there are no parameters or that the parameters
    /// all use the default format (text); or one, in which case the specified format code
    /// is applied to all parameters; or it can equal the actual number of parameters.
    pub params_format_len: i16,
    /// Int16[C] The parameter format codes. Each must presently be zero (text) or one (binary).
    pub params_format_code: I,

    /// The number of parameter values that follow (possibly zero).
    /// This must match the number of parameters needed by the query.
    pub params_len: i16,

    // Next, the following pair of fields appear for each parameter

    /// Int32 The length of the parameter value, in bytes (this count does not include itself). Can be zero.
    /// As a special case, -1 indicates a NULL parameter value. No value bytes follow in the NULL case
    ///
    /// followed by
    ///
    /// Byte[n] The value of the parameter, in the format indicated by the associated format code. n is the above length.
    pub params: P,

    // After the last parameter, the following fields appear:

    /// The number of result-column format codes that follow (denoted R below).
    /// This can be zero to indicate that there are no result columns or that the result
    /// columns should all use the default format (text); or one, in which case the
    /// specified format code is applied to all result columns (if any); or it can equal
    /// the actual number of result columns of the query.
    pub results_format_len: i16,

    /// Int16[R] The result-column format codes. Each must presently be zero (text) or one (binary).
    pub results_format_code: R,
}

impl<I,P,R> Bind<'_,I,P,R> {
    pub const FORMAT: u8 = b'B';
}

impl<'a, I,P,R> ProtocolEncode for Bind<'a,I,P,R>
where
    I: IntoIterator<Item = i16>,
    P: IntoIterator<Item = &'a crate::encode::Encoded<'a>>,
    R: IntoIterator<Item = i16>,
{
    fn encode(self, buf: &mut BytesMut) -> Result<(), ProtocolError> {
        let offset = buf.len();
        buf.put_u8(Self::FORMAT);

        // reserve for message length
        buf.put_u32(0);

        buf.put(self.portal_name);
        buf.put_u8(b'\0');

        buf.put(self.prepare_name);
        buf.put_u8(b'\0');

        buf.put_i16(self.params_format_len);
        for format_code in self.params_format_code {
            buf.put_i16(format_code);
        }

        buf.put_i16(self.params_len);
        for param in self.params {
            use crate::encode::{Encoded, ValueRef::*};
            // NOTE: idk how to properly abstract this,
            // number `to_be_bytes()` cannot be returned
            // from function
            match Encoded::value(&param) {
                Null => todo!("how to write NULL ?"),
                I32(num) => {
                    buf.put_i32(4);
                    buf.put_slice(&num.to_be_bytes());
                },
                Bool(b) => {
                    buf.put_i32(1);
                    buf.put_u8(*b as _);
                },
                Slice(items) => {
                    buf.put_i32(items.len() as _);
                    buf.put_slice(items);
                },
                Bytes(items) => {
                    buf.put_i32(items.len() as _);
                    buf.put_slice(items);
                }
            }
        }

        buf.put_i16(self.results_format_len);
        for format_code in self.results_format_code {
            buf.put_i16(format_code);
        }

        // write length, excluding msg format
        pg_write_len(&mut buf[offset + 1..])?;

        Ok(())
    }
}


/// Identifies the message as a Execute command
pub struct Execute<'a> {
    /// The name of the portal to execute (an empty string selects the unnamed portal).
    pub portal_name: &'a [u8],
    /// Maximum number of rows to return, if portal contains a query that returns rows
    /// (ignored otherwise). Zero denotes “no limit”.
    pub max_row: i32,
}

impl Execute<'_> {
    pub const FORMAT: u8 = b'E';
}

impl ProtocolEncode for Execute<'_> {
    fn encode(self, buf: &mut BytesMut) -> Result<(), ProtocolError> {
        let offset = buf.len();
        buf.put_u8(Self::FORMAT);

        // reserve for length
        buf.put_u32(0);

        buf.put(self.portal_name);
        buf.put_u8(b'\0');

        buf.put_i32(self.max_row);

        // write length, excluding msg format
        pg_write_len(&mut buf[offset + 1..])?;

        Ok(())
    }
}

