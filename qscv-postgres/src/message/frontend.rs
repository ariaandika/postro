//! Postgres Frontend Messages
use bytes::{BufMut, BytesMut};

use super::ext::{BufMutExt, StrExt, UsizeExt};

// Other Frontend Message:
// CancelRequest
// Close('C')
// CopyData('d')
// CopyDone('c')
// CopyFail('f')
// Describe('D')
// Flush('H')
// FunctionCall('F')
// GSSENCRequest
// GSSENCResponse('p')
// SASLInitialResponse('p')
// SASLResponse('p')
// SSLRequest
// Terminate('X')

/// write a frontend message to `buf`
///
/// to write multiple message at the same time, use [`write_batch`]
/// for better capacity reserve
pub fn write<F: FrontendProtocol>(msg: F, buf: &mut BytesMut) {
    // format + length
    const PREFIX: usize = 1 + 4;

    let size = msg.size_hint();
    buf.reserve(PREFIX + size as usize);

    let offset = buf.len();
    buf.put_u8(F::FORMAT);
    buf.put_i32(4 + size);

    msg.encode(&mut *buf);

    assert_eq!(
        buf[offset..].len(),
        PREFIX + size as usize,
        "[BUG] Frontend Message body not equal to size hint"
    );
}

/// a type which can be encoded into postgres frontend message
pub trait FrontendProtocol {
    /// message format
    const FORMAT: u8;

    /// size of the main body
    ///
    /// note that this is *only* the size of main body as oppose of actual postgres message
    fn size_hint(&self) -> i32;

    /// write the main body of the message
    ///
    /// `buf` have the length returned from `size_hint`
    ///
    /// writing less or past length results in panic
    fn encode(self, buf: impl BufMut);
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
impl Startup<'_> {
    pub fn write(self, buf: &mut BytesMut) {
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

        // write the length
        let mut written_buf = &mut buf[offset..];
        written_buf.put_i32(written_buf.len().to_i32());
    }
}

#[derive(Debug)]
pub struct PasswordMessage<'a> {
    pub password: &'a str,
}

impl FrontendProtocol for PasswordMessage<'_> {
    const FORMAT: u8 = b'p';

    fn size_hint(&self) -> i32 {
        self.password.nul_string_len()
    }

    fn encode(self, mut buf: impl BufMut) {
        buf.put_nul_string(self.password);
    }
}

/// Identifies the message as a simple query
pub struct Query<'a> {
    /// the query string itself
    pub sql: &'a str,
}

impl FrontendProtocol for Query<'_> {
    const FORMAT: u8 = b'Q';

    fn size_hint(&self) -> i32 {
        self.sql.nul_string_len()
    }

    fn encode(self, mut buf: impl BufMut) {
        buf.put_nul_string(self.sql);
    }
}

/// Identifies the message as a Parse command
pub struct Parse<'a,I> {
    /// prepared statement name (an empty string selects the unnamed prepared statement).
    pub prepare_name: &'a str,
    /// The query string to be parsed.
    pub sql: &'a str,
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

impl<I> FrontendProtocol for Parse<'_,I>
where
    I: IntoIterator<Item = i32>
{
    const FORMAT: u8 = b'P';

    fn size_hint(&self) -> i32 {
        self.prepare_name.nul_string_len() +
        self.sql.nul_string_len() +
        2 +
        (self.data_types_len as i32 * 4)
    }

    fn encode(self, mut buf: impl BufMut) {
        buf.put_nul_string(self.prepare_name);
        buf.put_nul_string(self.sql);
        buf.put_i16(self.data_types_len);
        for dt in self.data_types {
            buf.put_i32(dt);
        }
    }
}

/// Identifies the message as a Sync command
pub struct Sync;

impl FrontendProtocol for Sync {
    const FORMAT: u8 = b'S';

    fn size_hint(&self) -> i32 { 0 }

    fn encode(self, _: impl BufMut) { }
}

/// Identifies the message as a Bind command
pub struct Bind<'a,I,L,P,R> {
    /// The name of the destination portal (an empty string selects the unnamed portal).
    pub portal_name: &'a str,
    /// The name of the source prepared statement (an empty string selects the unnamed prepared statement).
    pub prepare_name: &'a str,

    /// The number of parameter format codes that follow (denoted C below).
    /// This can be zero to indicate that there are no parameters or that the parameters
    /// all use the default format (text); or one, in which case the specified format code
    /// is applied to all parameters; or it can equal the actual number of parameters.
    pub params_format_len: i16,
    /// Int16[C] The parameter format codes. Each must presently be zero (text) or one (binary).
    pub params_format_code: I,

    /// The number of parameter values that follow (possibly zero).
    /// This must match the number of parameters needed by the query.
    pub params_len: L,

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

// NOTE: idk how to properly abstract the Params,
// number `to_be_bytes()` cannot be returned
// from function

impl<'a,I,L,P,R> FrontendProtocol for Bind<'a,I,L,P,R>
where
    I: IntoIterator<Item = i16>,
    L: IntoIterator,
    L::IntoIter: ExactSizeIterator,
    P: IntoIterator<Item = &'a crate::encode::Encoded<'a>> + Copy/* expected a reference */,
    R: IntoIterator<Item = i16>,
{
    const FORMAT: u8 = b'B';

    fn size_hint(&self) -> i32 {
        self.portal_name.nul_string_len() +
        self.prepare_name.nul_string_len() +
        // self.params_format_len (i16)
        2 +
        // self.params_format_code (i16[])
        (self.params_format_len as i32 * 2) +
        // self.params_len (i16)
        2 +
        IntoIterator::into_iter(self.params)
            .fold(0i32, |acc,n|{
                use crate::encode::{Encoded, ValueRef::*};
                let len_and_data = match Encoded::value(&n) {
                    Null => todo!("what the length of NULL ?"),
                    I32(_) => 4 + 4,
                    Bool(_) => 4 + 1,
                    Slice(items) => 4 + items.len().to_i32(),
                    Bytes(items) => 4 + items.len().to_i32(),
                };
                acc + len_and_data
            }) +
        // self.results_format_len (i16)
        2 +
        // self.results_format_code (i16[])
        (self.results_format_len as i32 * 2)
    }

    fn encode(self, mut buf: impl BufMut) {
        buf.put_nul_string(self.portal_name);
        buf.put_nul_string(self.prepare_name);

        buf.put_i16(self.params_format_len);
        for format_code in self.params_format_code {
            buf.put_i16(format_code);
        }

        buf.put_i16(self.params_len.into_iter().len().to_i16());
        for param in self.params {
            use crate::encode::{Encoded, ValueRef::*};
            match Encoded::value(&param) {
                Null => todo!("how to write NULL ?"),
                I32(num) => {
                    buf.put_i32(4);
                    buf.put_i32(*num);
                },
                Bool(b) => {
                    buf.put_i32(1);
                    buf.put_u8(*b as _);
                },
                Slice(items) => {
                    buf.put_i32(items.len().to_i32());
                    buf.put_slice(items);
                },
                Bytes(items) => {
                    buf.put_i32(items.len().to_i32());
                    buf.put_slice(items);
                }
            }
        }

        buf.put_i16(self.results_format_len);
        for format_code in self.results_format_code {
            buf.put_i16(format_code);
        }
    }
}

/// Identifies the message as a Execute command
pub struct Execute<'a> {
    /// The name of the portal to execute (an empty string selects the unnamed portal).
    pub portal_name: &'a str,
    /// Maximum number of rows to return, if portal contains a query that returns rows
    /// (ignored otherwise). Zero denotes “no limit”.
    pub max_row: i32,
}

impl FrontendProtocol for Execute<'_> {
    const FORMAT: u8 = b'E';

    fn size_hint(&self) -> i32 {
        self.portal_name.nul_string_len() +
        // self.max_row
        4
    }

    fn encode(self, mut buf: impl BufMut) {
        buf.put_nul_string(self.portal_name);
        buf.put_i32(self.max_row);
    }
}

