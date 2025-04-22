//! Postgres Backend Messages
use bytes::{Buf, Bytes};

use super::ProtocolError;
use crate::ext::BytesExt;

/// A type that can be decoded into postgres backend message
pub trait BackendProtocol: Sized {
    fn decode(msgtype: u8, body: Bytes) -> Result<Self,ProtocolError>;
}

/// Postgres backend messages
#[derive(Debug)]
pub enum BackendMessage {
    Authentication(Authentication),
    BackendKeyData(BackendKeyData),
    BindComplete(BindComplete),
    CloseComplete(CloseComplete),
    CommandComplete(CommandComplete),
    DataRow(DataRow),
    ErrorResponse(ErrorResponse),
    EmptyQueryResponse(EmptyQueryResponse),
    NegotiateProtocolVersion(NegotiateProtocolVersion),
    NoData(NoData),
    NoticeResponse(NoticeResponse),
    ParameterDescription(ParameterDescription),
    ParameterStatus(ParameterStatus),
    ParseComplete(ParseComplete),
    PortalSuspended(PortalSuspended),
    ReadyForQuery(ReadyForQuery),
    RowDescription(RowDescription),
}

macro_rules! match_backend {
    ($($name:ident,)*) => {
        impl BackendMessage {
            pub fn msgtype(&self) -> u8 {
                match self {
                    $(Self::$name(_) => $name::MSGTYPE,)*
                }
            }

            /// Get message name from message type.
            ///
            /// Returns `"Unknown"` for unknown message type.
            pub fn message_name(msgtype: u8) -> &'static str {
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

// impl BackendMessage {
//     pub fn try_dberror(self) -> Result<Self, DatabaseError> {
//         match self {
//             Self::ErrorResponse(err) => Err(err.to_db_error()),
//             ok => Ok(ok),
//         }
//     }
// }

macro_rules! assert_msgtype {
    ($typ:ident) => {
        if Self::MSGTYPE != $typ {
            return Err(ProtocolError::unexpected(Self::MSGTYPE,$typ))
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

impl Authentication {
    pub const MSGTYPE: u8 = b'R';
}

impl BackendProtocol for Authentication {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(msgtype);
        let auth = match body.get_u32() {
            0 => Authentication::Ok,
            2 => Authentication::KerberosV5,
            3 => Authentication::CleartextPassword,
            5 => Authentication::MD5Password { salt: body.get_u32().to_be_bytes(), },
            7 => Authentication::GSS,
            8 => Authentication::GSSContinue { data: body },
            9 => Authentication::SSPI,
            10 => Authentication::SASL { name: body },
            11 => Authentication::SASLContinue { data: body },
            12 => Authentication::SASLFinal { data: body },
            auth => return Err(ProtocolError::unknown_auth(auth)),
        };
        Ok(auth)
    }
}

/// Identifies the message as cancellation key data.
///
/// The frontend must save these values if it wishes to be able to issue CancelRequest messages later.
#[derive(Debug)]
pub struct BackendKeyData {
    /// The process ID of this backend.
    pub process_id: u32,
    /// The secret key of this backend.
    pub secret_key: u32,
}

impl BackendKeyData {
    pub const MSGTYPE: u8 = b'K';
}

impl BackendProtocol for BackendKeyData {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self {
            process_id: body.get_u32(),
            secret_key: body.get_u32(),
        })
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
    pub const MSGTYPE: u8 = b'S';
}

impl BackendProtocol for ParameterStatus {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self {
            name: body.get_nul_string(),
            value: body.get_nul_string(),
        })
    }
}

/// A warning message. The frontend should display the message.
pub struct NoticeResponse {
    pub body: Bytes
}

impl NoticeResponse {
    pub const MSGTYPE: u8 = b'N';
}

impl BackendProtocol for NoticeResponse {
    fn decode(msgtype: u8, body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(NoticeResponse { body })
    }
}

/// Identifies the message as an error
///
/// The message body consists of one or more identified fields, followed by a zero byte as a terminator.
/// Fields can appear in any order.
///
/// For each field there is the following:
///
/// `Byte1` A code identifying the field type; if zero, this is the message terminator and no string follows.
/// The presently defined field types are listed in Section 53.8.
/// Since more field types might be added in future,
/// frontends should silently ignore fields of unrecognized type.
///
/// `String` The field value.
pub struct ErrorResponse {
    pub body: Bytes,
}

impl ErrorResponse {
    pub const MSGTYPE: u8 = b'E';
}

impl BackendProtocol for ErrorResponse {
    fn decode(msgtype: u8, body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self { body })
    }
}

/// Identifies the message as a row description
#[derive(Debug)]
pub struct RowDescription {
    /// Specifies the number of fields in a row (can be zero).
    pub field_len: u16,
    /// Undecoded response body.
    pub body: Bytes,
}

impl RowDescription {
    pub const MSGTYPE: u8 = b'T';
}

impl BackendProtocol for RowDescription {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self, ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self {
            field_len: body.get_u16(),
            body,
        })
    }
}

#[derive(Debug)]
/// Identifies the message as a data row.
pub struct DataRow {
    /// The number of column values that follow (possibly zero).
    pub column_len: u16,
    pub body: Bytes,
}

impl DataRow {
    pub const MSGTYPE: u8 = b'D';
}

impl BackendProtocol for DataRow {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self, ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self {
            column_len: body.get_u16(),
            body,
        })
    }
}

/// Identifies the message as a command-completed response
///
/// For an INSERT command, the tag is INSERT oid rows, where rows is the number of rows inserted.
/// oid used to be the object ID of the inserted row if rows was 1 and the target table had OIDs,
/// but OIDs system columns are not supported anymore; therefore oid is always 0.
///
/// For a DELETE command, the tag is DELETE rows where rows is the number of rows deleted.
///
/// For an UPDATE command, the tag is UPDATE rows where rows is the number of rows updated.
///
/// For a MERGE command, the tag is MERGE rows where rows is the number of rows inserted, updated, or deleted.
///
/// For a SELECT or CREATE TABLE AS command, the tag is SELECT rows where rows is the number of rows retrieved.
///
/// For a MOVE command, the tag is MOVE rows where rows is the number of rows
/// the cursor's position has been changed by.
///
/// For a FETCH command, the tag is FETCH rows where rows is the number of rows that have
/// been retrieved from the cursor.
///
/// For a COPY command, the tag is COPY rows where rows is the number of rows copied.
/// (Note: the row count appears only in PostgreSQL 8.2 and later.)
#[derive(Debug)]
pub struct CommandComplete {
    /// The command tag. This is usually a single word that identifies which SQL command was completed.
    pub tag: Bytes,
}

impl CommandComplete {
    pub const MSGTYPE: u8 = b'C';
}

impl BackendProtocol for CommandComplete {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self, ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self {
            tag: body.get_nul_bytes(),
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
    /// Then, for protocol option not recognized by the server, there is the following:
    pub opt_names: Bytes,
}

impl NegotiateProtocolVersion {
    pub const MSGTYPE: u8 = b'v';
}

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
    /// Then, for each parameter, there is the following:
    ///
    /// Specifies the object ID of the parameter data type.
    pub oids: Bytes,
}

impl ParameterDescription  {
    pub const MSGTYPE: u8 = b't';
}

impl BackendProtocol for ParameterDescription {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(msgtype);
        Ok(Self {
            param_len: body.get_u16(),
            oids: body,
        })
    }
}

macro_rules! unit_msg {
    ($(
        $(#[$doc:meta])* struct $name:ident, $ty:literal;
    )*) => {$(
            $(#[$doc])*
            #[derive(Debug)]
            pub struct $name;

            impl $name {
                pub const MSGTYPE: u8 = $ty;
            }

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

    /// Identifies the message type. ReadyForQuery is sent whenever the backend is ready for a new query cycle.
    struct ReadyForQuery, b'Z';
}

