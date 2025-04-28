//! Postgres Backend Messages
//!
//! <https://www.postgresql.org/docs/current/protocol-message-formats.html>
use bytes::{Buf, Bytes};

use super::ProtocolError;
use crate::{common::ByteStr, ext::BytesExt};

/// A type that can be decoded into postgres backend message.
pub trait BackendProtocol: Sized + std::fmt::Debug {
    /// Try decode given bytes into message.
    ///
    /// Note that `body` is only the main body, **excluding** message type and length.
    fn decode(msgtype: u8, body: Bytes) -> Result<Self, ProtocolError>;
}

/// Postgres backend messages.
pub enum BackendMessage {
    /// Identifies the message as an authentication request.
    Authentication(Authentication),
    /// Identifies the message as cancellation key data.
    BackendKeyData(BackendKeyData),
    /// Identifies the message as a Bind-complete indicator.
    BindComplete(BindComplete),
    /// Identifies the message as a Close-complete indicator.
    CloseComplete(CloseComplete),
    /// Identifies the message as a command-completed response.
    CommandComplete(CommandComplete),
    /// Identifies the message as a data row.
    DataRow(DataRow),
    /// Identifies the message as an error.
    ErrorResponse(ErrorResponse),
    /// Identifies the message as a response to an empty query string.
    EmptyQueryResponse(EmptyQueryResponse),
    /// Identifies the message as a protocol version negotiation message.
    NegotiateProtocolVersion(NegotiateProtocolVersion),
    /// Identifies the message as a no-data indicator.
    NoData(NoData),
    /// Identifies the message as a notice.
    NoticeResponse(NoticeResponse),
    /// Identifies the message as a parameter description.
    ParameterDescription(ParameterDescription),
    /// Identifies the message as a run-time parameter status report
    ParameterStatus(ParameterStatus),
    /// Identifies the message as a Parse-complete indicator.
    ParseComplete(ParseComplete),
    /// Identifies the message as a portal-suspended indicator.
    PortalSuspended(PortalSuspended),
    /// Identifies the message type. ReadyForQuery is sent whenever the backend is ready for a new query cycle.
    ReadyForQuery(ReadyForQuery),
    /// Identifies the message as a row description
    RowDescription(RowDescription),
}

macro_rules! match_backend {
    ($($name:ident,)*) => {
        impl BackendMessage {
            /// Returns the message type.
            pub const fn msgtype(&self) -> u8 {
                match self {
                    $(Self::$name(_) => $name::MSGTYPE,)*
                }
            }

            /// Get message name from message type.
            ///
            /// Returns `"Unknown"` for unknown message type.
            pub const fn message_name(msgtype: u8) -> &'static str {
                match msgtype {
                    $($name::MSGTYPE => stringify!($name),)*
                    _ => "Unknown",
                }
            }
        }
        impl BackendProtocol for BackendMessage {
            fn decode(msgtype: u8, body: Bytes) -> Result<Self, ProtocolError> {
                let message = match msgtype {
                    $($name::MSGTYPE => Self::$name(<$name as BackendProtocol>::decode(msgtype, body)?),)*
                    _ => return Err(ProtocolError::unknown(msgtype)),
                };
                Ok(message)
            }
        }
        impl std::fmt::Debug for BackendMessage {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    $(Self::$name(e) => std::fmt::Debug::fmt(e, f),)*
                }
            }
        }
    };
}


match_backend! {
    Authentication,
    BackendKeyData,
    BindComplete,
    CloseComplete,
    CommandComplete,
    DataRow,
    ErrorResponse,
    EmptyQueryResponse,
    NegotiateProtocolVersion,
    NoData,
    NoticeResponse,
    ParameterDescription,
    ParameterStatus,
    ParseComplete,
    PortalSuspended,
    ReadyForQuery,
    RowDescription,
}

macro_rules! assert_msgtype {
    ($typ:ident) => {
        if Self::MSGTYPE != $typ {
            return Err(ProtocolError::unexpected(Self::MSGTYPE,$typ))
        }
    };
}

macro_rules! msgtype {
    ($me:ident,$ty:literal) => {
        impl $me {
            #[doc = concat!("`",stringify!($ty),"`")]
            pub const MSGTYPE: u8 = $ty;
        }
    };
}

/// Identifies the message as an authentication request.
#[derive(Debug)]
pub enum Authentication {
    /// Specifies that the authentication was successful.
    Ok,
    /// Specifies that Kerberos V5 authentication is required.
    KerberosV5,
    /// Specifies that a clear-text password is required.
    CleartextPassword,
    /// Specifies that an MD5-encrypted password is required.
    MD5Password {
        /// The salt to use when encrypting the password.
        salt: [u8;4],
    },
    /// Specifies that GSSAPI authentication is required.
    GSS,
    /// GSSAPI or SSPI authentication data.
    GSSContinue {
        data: Bytes,
    },
    /// Specifies that SSPI authentication is required.
    SSPI,
    /// Specifies that SASL authentication is required.
    SASL {
        /// The message body is a list of SASL authentication mechanisms, in the server's order of preference.
        ///
        /// A zero byte is required as terminator after the last authentication mechanism name.
        ///
        /// For each mechanism, there is the following:
        name: Bytes,
    },
    /// Specifies that this message contains a SASL challenge.
    SASLContinue {
        /// SASL data, specific to the SASL mechanism being used.
        data: Bytes,
    },
    /// Specifies that SASL authentication has completed.
    SASLFinal {
        /// SASL outcome "additional data", specific to the SASL mechanism being used.
        data: Bytes,
    },
}

msgtype!(Authentication, b'R');

impl BackendProtocol for Authentication {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(msgtype);
        let auth = match body.get_u32() {
            0 => Self::Ok,
            2 => Self::KerberosV5,
            3 => Self::CleartextPassword,
            5 => Self::MD5Password { salt: body.get_u32().to_be_bytes(), },
            7 => Self::GSS,
            8 => Self::GSSContinue { data: body },
            9 => Self::SSPI,
            10 => Self::SASL { name: body },
            11 => Self::SASLContinue { data: body },
            12 => Self::SASLFinal { data: body },
            auth => panic!("Unknown Authentication type: \"{auth}\""),
        };
        Ok(auth)
    }
}

/// Identifies the message as cancellation key data.
///
/// The frontend must save these values if it wishes to be able to issue CancelRequest messages later.
pub struct BackendKeyData {
    /// The process ID of this backend.
    pub process_id: u32,
    /// The secret key of this backend.
    pub secret_key: u32,
}

msgtype!(BackendKeyData, b'K');

impl BackendProtocol for BackendKeyData {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self {
            process_id: body.get_u32(),
            secret_key: body.get_u32(),
        })
    }
}

/// Identifies the message as a run-time parameter status report.
#[derive(Debug)]
pub struct ParameterStatus {
    /// The name of the run-time parameter being reported.
    pub name: ByteStr,
    /// The current value of the parameter.
    pub value: ByteStr,
}

msgtype!(ParameterStatus, b'S');

impl BackendProtocol for ParameterStatus {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self {
            name: body.get_nul_bytestr()?,
            value: body.get_nul_bytestr()?,
        })
    }
}

/// Identifies the message as a notice.
pub struct NoticeResponse {
    /// Raw message body.
    ///
    /// The message body consists of one or more identified fields, followed by a zero byte as a terminator.
    ///
    /// Fields can appear in any order.
    ///
    /// For each field there is the following:
    ///
    /// - `Byte1` A code identifying the field type; if zero, this is the message terminator and no string follows.
    ///
    /// The presently defined field types are listed in [Section 53.8][53_8].
    ///
    /// Since more field types might be added in future, frontends should silently ignore fields of unrecognized type.
    ///
    /// - `String` The field value.
    ///
    /// [53_8]: https://www.postgresql.org/docs/current/protocol-error-fields.html
    pub body: Bytes
}

msgtype!(NoticeResponse, b'N');

impl NoticeResponse {
    pub fn new(body: Bytes) -> Self {
        Self { body }
    }
}

impl BackendProtocol for NoticeResponse {
    fn decode(msgtype: u8, body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self { body })
    }
}

/// Identifies the message as an error.
pub struct ErrorResponse {
    /// Raw message body.
    ///
    /// The message body consists of one or more identified fields, followed by a zero byte as a terminator.
    /// Fields can appear in any order.
    ///
    /// For each field there is the following:
    ///
    /// - `Byte1` A code identifying the field type; if zero, this is the message terminator and no string follows.
    ///
    /// The presently defined field types are listed in [Section 53.8][53_8].
    ///
    /// Since more field types might be added in future, frontends should silently ignore fields of unrecognized type.
    ///
    /// - `String` The field value.
    ///
    /// [53_8]: https://www.postgresql.org/docs/current/protocol-error-fields.html
    pub body: Bytes,
}

msgtype!(ErrorResponse, b'E');

impl ErrorResponse {
    pub fn new(body: Bytes) -> Self {
        Self { body }
    }
}

impl BackendProtocol for ErrorResponse {
    fn decode(msgtype: u8, body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self { body })
    }
}

/// Identifies the message as a row description
pub struct RowDescription {
    /// Raw message body.
    ///
    /// - `Int16` Specifies the number of fields in a row (can be zero).
    ///
    /// For each field, there is the following:
    ///
    /// - `String` The field name.
    /// - `Int32` If the field can be identified as a column of a specific table,
    ///   the object ID of the table; otherwise zero.
    /// - `Int16` If the field can be identified as a column of a specific table,
    ///   the attribute number of the column; otherwise zero.
    /// - `Int32` The object ID of the field's data type.
    /// - `Int16` The data type size (see pg_type.typlen). Note that negative values denote variable-width types.
    /// - `Int32` The type modifier (see pg_attribute.atttypmod). The meaning of the modifier is type-specific.
    /// - `Int16` The format code being used for the field. Currently will be zero (text) or one (binary).
    ///   In a RowDescription returned from the statement variant of Describe,
    ///   the format code is not yet known and will always be zero.
    pub body: Bytes,
}

msgtype!(RowDescription, b'T');

impl BackendProtocol for RowDescription {
    fn decode(msgtype: u8, body: Bytes) -> Result<Self, ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self { body })
    }
}

/// Identifies the message as a data row.
pub struct DataRow {
    /// Raw row buffer.
    ///
    /// - `Int16` The number of column values that follow (possibly zero).
    ///
    /// Next, the following pair of fields appear for each column:
    ///
    /// - `Int32` The length of the column value, in bytes (this count does not include itself).
    ///
    /// Can be zero. As a special case, -1 indicates a NULL column value. No value bytes follow in the NULL case.
    ///
    /// - `Byte[n]` The value of the column, in the format indicated by the associated format code.
    pub body: Bytes,
}

msgtype!(DataRow, b'D');

impl BackendProtocol for DataRow {
    fn decode(msgtype: u8, body: Bytes) -> Result<Self, ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self { body })
    }
}

/// Identifies the message as a command-completed response.
#[derive(Debug)]
pub struct CommandComplete {
    /// The command tag. This is usually a single word that identifies which SQL command was completed.
    ///
    /// For an `INSERT` command, the tag is `INSERT oid rows`, where `rows` is the number of rows inserted.
    /// `oid` used to be the object ID of the inserted row if `rows` was 1 and the target table had OIDs,
    /// but OIDs system columns are not supported anymore; therefore `oid` is always 0.
    ///
    /// For a `DELETE` command, the tag is `DELETE rows` where `rows` is the number of rows deleted.
    ///
    /// For an `UPDATE` command, the tag is `UPDATE rows` where `rows` is the number of rows updated.
    ///
    /// For a `MERGE` command, the tag is `MERGE rows` where `rows` is the
    /// number of rows inserted, updated, or deleted.
    ///
    /// For a `SELECT` or `CREATE TABLE AS` command, the tag is `SELECT rows`
    /// where `rows` is the number of rows retrieved.
    ///
    /// For a `MOVE` command, the tag is `MOVE rows` where `rows` is the number of rows
    /// the cursor's position has been changed by.
    ///
    /// For a `FETCH` command, the tag is `FETCH rows` where `rows` is the number of rows that have
    /// been retrieved from the cursor.
    ///
    /// For a `COPY` command, the tag is `COPY rows` where `rows` is the number of rows copied.
    /// (Note: the row count appears only in PostgreSQL 8.2 and later.)
    pub tag: ByteStr,
}

msgtype!(CommandComplete, b'C');

impl BackendProtocol for CommandComplete {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self, ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self {
            tag: body.get_nul_bytestr()?,
        })
    }
}

/// Identifies the message as a protocol version negotiation message.
#[derive(Debug)]
pub struct NegotiateProtocolVersion {
    /// Newest minor protocol version supported by the server for the major protocol version requested by the client.
    pub minor: u32,
    /// Number of protocol options not recognized by the server.
    pub len: u32,
    /// Raw buffer for option not recognized by the server.
    ///
    /// There is the following:
    ///
    /// - `String` The option name.
    pub opt_names: Bytes,
}

msgtype!(NegotiateProtocolVersion, b'v');

impl BackendProtocol for NegotiateProtocolVersion {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self {
            minor: body.get_u32(),
            len: body.get_u32(),
            opt_names: body,
        })
    }
}

/// Identifies the message as a parameter description.
#[derive(Debug)]
pub struct ParameterDescription {
    /// The number of parameters used by the statement (can be zero).
    pub param_len: u16,
    /// Raw buffer for message body.
    ///
    /// For each parameter, there is the following:
    ///
    /// - `Int32` Specifies the object ID of the parameter data type.
    pub oids: Bytes,
}

msgtype!(ParameterDescription, b't');

impl BackendProtocol for ParameterDescription {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self {
            param_len: body.get_u16(),
            oids: body,
        })
    }
}

/// Identifies the message type. ReadyForQuery is sent whenever the backend is ready for a new query cycle.
pub struct ReadyForQuery {
    /// Current backend transaction status indicator.
    ///
    /// Possible values are 'I' if idle (not in a transaction block);
    /// 'T' if in a transaction block;
    /// or 'E' if in a failed transaction block (queries will be rejected until block is ended).
    pub tx_status: u8
}

msgtype!(ReadyForQuery, b'Z');

impl BackendProtocol for ReadyForQuery {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self { tx_status: body.get_u8() })
    }
}

macro_rules! unit_msg {
    ($(
        $(#[$doc:meta])* struct $name:ident, $ty:literal;
    )*) => {$(
            $(#[$doc])*
            #[derive(Debug)]
            pub struct $name;

            msgtype!($name, $ty);

            impl BackendProtocol for $name {
                fn decode(msgtype: u8, _: Bytes) -> Result<Self,ProtocolError> {
                    if $name::MSGTYPE != msgtype {
                        return Err(ProtocolError::unexpected(Self::MSGTYPE,msgtype))
                    }
                    Ok(Self)
                }
            }
    )*};
}

unit_msg! {
    /// Identifies the message as a Bind-complete indicator.
    struct BindComplete, b'2';

    /// Identifies the message as a Close-complete indicator.
    struct CloseComplete, b'3';

    /// Identifies the message as a response to an empty query string.
    ///
    /// This substitutes for CommandComplete.
    struct EmptyQueryResponse, b'I';

    /// Identifies the message as a no-data indicator.
    struct NoData, b'n';

    /// Identifies the message as a Parse-complete indicator.
    struct ParseComplete, b'1';

    /// Identifies the message as a portal-suspended indicator.
    ///
    /// Note this only appears if an Execute message's row-count limit was reached.
    struct PortalSuspended, b's';
}

// CUSTOM DEBUG

impl std::fmt::Debug for BackendKeyData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackendKeyData")
            .field("process_id", &self.process_id)
            .field("secret_key", &"<REDACTED>")
            .finish()
    }
}

impl std::fmt::Debug for ReadyForQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadyForQuery")
            .field("tx_status", &match self.tx_status {
                b'I' => "Idle(I)",
                b'T' => "Transaction(T)",
                b'E' => "FailedTx(E)",
                _ => "unknown",
            })
            .finish()
    }
}

impl std::fmt::Debug for RowDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RowDescription")
            .field("body", &"<BINARY>")
            .finish()
    }
}

impl std::fmt::Debug for DataRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataRow")
            .field("body", &"<BINARY>")
            .finish()
    }
}

