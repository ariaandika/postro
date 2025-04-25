//! Postgres Frontend Messages
//!
//! <https://www.postgresql.org/docs/current/protocol-message-formats.html>
use bytes::{BufMut, BytesMut};

use super::{Oid, PgFormat};
use crate::ext::{BindParams, BufMutExt, StrExt, UsizeExt};

/// Write a frontend message to `buf`.
pub fn write<F: FrontendProtocol>(msg: F, buf: &mut BytesMut) {
    // msgtype + length
    const PREFIX: usize = 1 + 4;

    let size_hint = msg.size_hint();
    buf.reserve(PREFIX + size_hint as usize);

    let offset = buf.len();
    buf.put_u8(F::MSGTYPE);
    buf.put_u32(4 + size_hint);

    msg.encode(&mut *buf);

    assert_eq!(
        buf.len() - offset,
        PREFIX + size_hint as usize,
        "Frontend message body size not equal to size hint"
    );
}

/// A type which can be encoded into postgres frontend message
pub trait FrontendProtocol {
    /// Message type.
    const MSGTYPE: u8;

    /// Size of the main body.
    ///
    /// Note that this is **only** the size of main body as oppose of actual postgres message which
    /// include the length itself.
    fn size_hint(&self) -> u32;

    /// Write the main body of the message.
    ///
    /// The lenght of body written must be equal to the
    /// length returned by [`size_hint`][FrontendProtocol::size_hint].
    fn encode(self, buf: impl BufMut);
}

/// Postgres Startup frontend message
///
/// For historical reasons, the very first message sent by the client (the [`Startup`] message)
/// has no initial message-type byte, thus [`Startup`] does not implement [`FrontendProtocol`].
///
/// To write startup message, use [`Startup::write`].
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

impl Startup<'_> {
    pub fn write(self, buf: &mut BytesMut) {
        let offset = buf.len();

        // Length of message contents in bytes, including self.
        // reserve 4 bytes for length
        buf.put_u32(0);

        // Int32(196608)
        // The protocol version number.
        // The most significant 16 bits are the major version number (3 for the protocol described here).
        // The least significant 16 bits are the minor version number (0 for the protocol described here).
        buf.put_u32(196_608);

        // The protocol version number is followed by one or more pairs of parameter name and value strings.

        // user: The database user name to connect as. Required; there is no default.

        buf.put_nul_string("user");
        buf.put_nul_string(self.user);

        // database: The database to connect to. Defaults to the user name.

        if let Some(db) = self.database {
            buf.put_nul_string("database");
            buf.put_nul_string(db);
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
            buf.put_nul_string("replication");
            buf.put_nul_string(repl);
        }

        // In addition to the above, other parameters may be listed.
        // Parameter names beginning with _pq_. are reserved for use as protocol extensions,
        // while others are treated as run-time parameters to be set at backend start time.
        // Such settings will be applied during backend start
        // (after parsing the command-line arguments if any) and will act as session defaults.

        // A zero byte is required as a terminator after the last name/value pair.
        buf.put_u8(b'\0');

        // write the length
        let mut written_buf = &mut buf[offset..];
        written_buf.put_u32(written_buf.len().to_u32());
    }
}

macro_rules! size_of {
    ($s1:tt.$f1:ident as $t1:ty, in ..$s2:tt.$f2:ident) => {
        ($s2.$f2 as u32 * u32::try_from(size_of::<$t1>()).expect("data type size too large for postgres"))
    };
    ($self:tt.$field:ident) => {
        u32::try_from(size_of_val(&$self.$field)).expect("data type size too large for postgres")
    };
}

/// Identifies the message as a Parse-complete indicator.
#[derive(Debug)]
pub struct PasswordMessage<'a> {
    /// The password (encrypted, if requested)
    pub password: &'a str,
}

impl FrontendProtocol for PasswordMessage<'_> {
    const MSGTYPE: u8 = b'p';

    fn size_hint(&self) -> u32 {
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
    const MSGTYPE: u8 = b'Q';

    fn size_hint(&self) -> u32 {
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
    pub oids_len: u16,
    /// Specifies the object ID of the parameter data type.
    ///
    /// Placing a zero here is equivalent to leaving the type unspecified.
    pub oids: I,
}

impl<I> FrontendProtocol for Parse<'_,I>
where
    I: IntoIterator<Item = Oid>
{
    const MSGTYPE: u8 = b'P';

    fn size_hint(&self) -> u32 {
        self.prepare_name.nul_string_len()
            + self.sql.nul_string_len()
            + size_of!(self.oids_len)
            + size_of!(self.oids as Oid, in ..self.oids_len)
    }

    fn encode(self, mut buf: impl BufMut) {
        buf.put_nul_string(self.prepare_name);
        buf.put_nul_string(self.sql);
        buf.put_u16(self.oids_len);
        for oid in self.oids {
            buf.put_u32(oid);
        }
    }
}

/// Identifies the message as a Sync command
pub struct Sync;

impl FrontendProtocol for Sync {
    const MSGTYPE: u8 = b'S';

    fn size_hint(&self) -> u32 { 0 }

    fn encode(self, _: impl BufMut) { }
}

/// Identifies the message as a Flush command
pub struct Flush;

impl FrontendProtocol for Flush {
    const MSGTYPE: u8 = b'H';

    fn size_hint(&self) -> u32 { 0 }

    fn encode(self, _: impl BufMut) { }
}

/// Identifies the message as a Bind command.
pub struct Bind<'a, ParamFmts, Params, ResultFmts> {
    /// The name of the destination portal (an empty string selects the unnamed portal).
    pub portal_name: &'a str,
    /// The name of the source prepared statement (an empty string selects the unnamed prepared statement).
    pub stmt_name: &'a str,

    /// The number of parameter format codes that follow.
    ///
    /// This can be zero to indicate that there are no parameters or that the parameters
    /// all use the default format (text); or one, in which case the specified format code
    /// is applied to all parameters; or it can equal the actual number of parameters.
    pub param_formats_len: u16,

    /// The parameter [`PgFormat`].
    pub param_formats: ParamFmts,

    /// The number of parameter values that follow (possibly zero).
    ///
    /// This must match the number of parameters needed by the query.
    pub params_len: u16,

    /// This is not postgres field, but a workaround helper to get `size_hint` easily.
    pub params_size_hint: u32,

    /// Int32 The length of the parameter value, in bytes (this count does not include itself). Can be zero.
    /// As a special case, -1 indicates a NULL parameter value. No value bytes follow in the NULL case
    ///
    /// followed by
    ///
    /// The value of the parameter, in the format indicated by the associated format code. n is the above length.
    pub params: Params,

    /// The number of result-column format codes that follow.
    ///
    /// This can be zero to indicate that there are no result columns or that the result
    /// columns should all use the default format (text); or one, in which case the
    /// specified format code is applied to all result columns (if any); or it can equal
    /// the actual number of result columns of the query.
    pub result_formats_len: u16,

    /// The result-columns [`PgFormat`].
    pub result_formats: ResultFmts,
}

impl<ParamFmts, Params, ResultFmts> FrontendProtocol for Bind<'_, ParamFmts, Params, ResultFmts>
where
    ParamFmts: IntoIterator<Item = PgFormat>,
    Params: Iterator + ExactSizeIterator,
    <Params as Iterator>::Item: BindParams,
    ResultFmts: IntoIterator<Item = PgFormat>,
{
    const MSGTYPE: u8 = b'B';

    fn size_hint(&self) -> u32 {
        self.portal_name.nul_string_len()
            + self.stmt_name.nul_string_len()
            + size_of!(self.param_formats_len)
            + size_of!(self.param_formats as u16, in ..self.param_formats_len)
            + size_of!(self.params_len)
            + self.params_size_hint
            + size_of!(self.result_formats_len)
            + size_of!(self.result_formats as u16, in ..self.result_formats_len)
    }

    fn encode(self, mut buf: impl BufMut) {
        buf.put_nul_string(self.portal_name);
        buf.put_nul_string(self.stmt_name);

        buf.put_u16(self.param_formats_len);
        for format in self.param_formats {
            buf.put_u16(format.format_code());
        }

        buf.put_u16(self.params_len);
        for param in self.params {
            // can be -1 for NULL
            buf.put_i32(param.size());
            buf.put(param);
        }

        buf.put_u16(self.result_formats_len);
        for format in self.result_formats {
            buf.put_u16(format.format_code());
        }
    }
}

/// Identifies the message as a Execute command
pub struct Execute<'a> {
    /// The name of the portal to execute (an empty string selects the unnamed portal).
    pub portal_name: &'a str,
    /// Maximum number of rows to return, if portal contains a query that returns rows
    /// (ignored otherwise). Zero denotes “no limit”.
    pub max_row: u32,
}

impl FrontendProtocol for Execute<'_> {
    const MSGTYPE: u8 = b'E';

    fn size_hint(&self) -> u32 {
        self.portal_name.nul_string_len() + size_of!(self.max_row)
    }

    fn encode(self, mut buf: impl BufMut) {
        buf.put_nul_string(self.portal_name);
        buf.put_u32(self.max_row);
    }
}

/// Identifies the message as a Close command
pub struct Close<'a> {
    /// 'S' to close a prepared statement; or 'P' to close a portal.
    pub variant: u8,
    /// The name of the prepared statement or portal to close
    /// (an empty string selects the unnamed prepared statement or portal).
    pub name: &'a str,
}

impl FrontendProtocol for Close<'_> {
    const MSGTYPE: u8 = b'C';

    fn size_hint(&self) -> u32 {
        size_of!(self.variant) + self.name.nul_string_len()
    }

    fn encode(self, mut buf: impl BufMut) {
        buf.put_u8(self.variant);
        buf.put_nul_string(self.name);
    }
}

/// Identifies the message as a Describe command.
pub struct Describe<'a> {
    /// 'S' to describe a prepared statement; or 'P' to describe a portal.
    pub kind: u8,
    /// The name of the prepared statement or portal to describe
    /// (an empty string selects the unnamed prepared statement or portal).
    pub name: &'a str,
}

impl FrontendProtocol for Describe<'_> {
    const MSGTYPE: u8 = b'D';

    fn size_hint(&self) -> u32 {
        size_of!(self.kind) + self.name.nul_string_len()
    }

    fn encode(self, mut buf: impl BufMut) {
        buf.put_u8(self.kind);
        buf.put_nul_string(self.name);
    }
}


