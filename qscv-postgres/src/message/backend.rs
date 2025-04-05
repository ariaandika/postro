use bytes::{Buf, Bytes};

use super::{authentication::Authentication, error::ProtocolError};
use crate::{
    common::{general, BytesRef},
    row_buffer::RowBuffer,
};

/// a type that can be decoded into postgres backend message
pub trait BackendProtocol: Sized {
    fn decode(msgtype: u8, body: Bytes) -> Result<Self,ProtocolError>;
}

macro_rules! assert_msgtype {
    ($self:ident,$typ:ident) => {
        // TODO: return error instead
        assert_eq!($self::MSGTYPE,$typ);
    };
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

/// postgres backend messages
#[derive(Debug)]
pub enum BackendMessage {
    Authentication(Authentication),
    BackendKeyData(BackendKeyData),
    ErrorResponse(ErrorResponse),
    ParameterStatus(ParameterStatus),
    ReadyForQuery(ReadyForQuery),
    RowDescription(RowDescription),
    DataRow(DataRow),
    CommandComplete(CommandComplete),
    ParseComplete(ParseComplete),
    BindComplete(BindComplete),
}

impl BackendProtocol for BackendMessage {
    fn decode(msgtype: u8, body: Bytes) -> Result<Self,ProtocolError> {
        let message = match msgtype {
            Authentication::MSGTYPE => Self::Authentication(<Authentication as BackendProtocol>::decode(msgtype, body)?),
            BackendKeyData::MSGTYPE => Self::BackendKeyData(<BackendKeyData as BackendProtocol>::decode(msgtype, body)?),
            ErrorResponse::MSGTYPE => Self::ErrorResponse(<ErrorResponse as BackendProtocol>::decode(msgtype, body)?),
            ParameterStatus::MSGTYPE => Self::ParameterStatus(<ParameterStatus as BackendProtocol>::decode(msgtype, body)?),
            ReadyForQuery::MSGTYPE => Self::ReadyForQuery(<ReadyForQuery as BackendProtocol>::decode(msgtype, body)?),
            RowDescription::MSGTYPE => Self::RowDescription(<RowDescription as BackendProtocol>::decode(msgtype, body)?),
            DataRow::MSGTYPE => Self::DataRow(<DataRow as BackendProtocol>::decode(msgtype, body)?),
            CommandComplete::MSGTYPE => Self::CommandComplete(<CommandComplete as BackendProtocol>::decode(msgtype, body)?),
            ParseComplete::MSGTYPE => Self::ParseComplete(<ParseComplete as BackendProtocol>::decode(msgtype, body)?),
            BindComplete::MSGTYPE => Self::BindComplete(<BindComplete as BackendProtocol>::decode(msgtype, body)?),
            _ => return Err(ProtocolError::new(general!(
                "unsupported backend message {:?}",
                BytesRef(&[msgtype])
            ))),
        };

        Ok(message)
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
    pub const MSGTYPE: u8 = b'K';
}

impl BackendProtocol for BackendKeyData {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(Self,msgtype);
        Ok(Self {
            process_id: body.get_i32(),
            secret_key: body.get_i32(),
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
    pub const FORMAT: u8 = b'S';
    pub const MSGTYPE: u8 = b'S';
}

impl BackendProtocol for ParameterStatus {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(Self,msgtype);
        Ok(Self {
            name: nul_string!(body),
            value: nul_string!(body),
        })
    }
}

#[derive(Debug)]
pub struct ReadyForQuery;

impl ReadyForQuery {
    pub const FORMAT: u8 = b'Z';
    pub const MSGTYPE: u8 = b'Z';
}

impl BackendProtocol for ReadyForQuery {
    fn decode(msgtype: u8, _: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(Self,msgtype);
        Ok(Self)
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

impl ErrorResponse {
    pub const FORMAT: u8 = b'E';
    pub const MSGTYPE: u8 = b'E';
}

impl BackendProtocol for ErrorResponse {
    fn decode(msgtype: u8, mut bytes: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(Self,msgtype);
        let mut body = std::collections::HashMap::new();
        loop {
            let f = bytes.get_u8();
            if f == b'\0' {
                break
            }
            let msg = nul_string!(bytes);
            body.insert(f, msg);
        }
        Ok(Self { body })
    }
}

#[derive(Debug)]
/// Identifies the message as a row description
pub struct RowDescription {
    /// Specifies the number of fields in a row (can be zero).
    pub field_len: i16,
    pub field_name: String,
    pub table_oid: i32,
    pub attribute_len: i16,
    pub data_type: i32,
    pub data_type_size: i16,
    pub type_modifier: i32,
    pub format_code: i16,
}

impl RowDescription {
    pub const FORMAT: u8 = b'T';
    pub const MSGTYPE: u8 = b'T';
}

impl BackendProtocol for RowDescription {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(Self,msgtype);
        Ok(Self {
            // Int16 Specifies the number of fields in a row (can be zero).
            field_len: body.get_i16(),
            // Int16 Specifies the number of fields in a row (can be zero).
            field_name: nul_string!(body),
            // If the field can be identified as a column of a specific table,
            // the object ID of the table; otherwise zero
            table_oid: body.get_i32(),
            // If the field can be identified as a column of a specific table,
            // the attribute number of the column; otherwise zero.
            attribute_len: body.get_i16(),
            // The object ID of the field's data type.
            data_type: body.get_i32(),
            // The data type size (see pg_type.typlen).
            // Note that negative values denote variable-width types.
            data_type_size: body.get_i16(),
            // The type modifier (see pg_attribute.atttypmod).
            // The meaning of the modifier is type-specific.
            type_modifier: body.get_i32(),
            // The format code being used for the field.
            // Currently will be zero (text) or one (binary).
            // In a RowDescription returned from the statement variant of Describe,
            // the format code is not yet known and will always be zero.
            format_code: body.get_i16(),
        })
    }
}

#[derive(Debug)]
/// Identifies the message as a row description
pub struct DataRow {
    pub row_buffer: RowBuffer,
}

impl DataRow {
    pub const FORMAT: u8 = b'D';
    pub const MSGTYPE: u8 = b'D';
}

impl BackendProtocol for DataRow {
    fn decode(msgtype: u8, mut body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(Self,msgtype);

        // The number of column values that follow (possibly zero).
        let col_values_len = body.get_i16();

        // lazily decode row without allocating `Vec`
        let row_buffer = RowBuffer::new(col_values_len, body);

        Ok(Self { row_buffer })
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
    pub tag: String,
}

impl CommandComplete {
    pub const FORMAT: u8 = b'C';
    pub const MSGTYPE: u8 = b'C';
}

impl BackendProtocol for CommandComplete {
    fn decode(msgtype: u8, body: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(Self,msgtype);
        Ok(Self {
            tag: match String::from_utf8(body.into()) {
                Ok(ok) => ok,
                Err(err) => return Err(ProtocolError::new(general!(
                    "non UTF-8 string in ParameterStatus: {err}",
                ))),
            }
        })
    }
}

#[derive(Debug)]
pub struct ParseComplete;

impl ParseComplete {
    pub const FORMAT: u8 = b'1';
    pub const MSGTYPE: u8 = b'1';
}

impl BackendProtocol for ParseComplete {
    fn decode(msgtype: u8, _: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(Self,msgtype);
        Ok(Self)
    }
}


#[derive(Debug)]
pub struct BindComplete;

impl BindComplete {
    pub const FORMAT: u8 = b'2';
    pub const MSGTYPE: u8 = b'2';
}

impl BackendProtocol for BindComplete {
    fn decode(msgtype: u8, _: Bytes) -> Result<Self,ProtocolError> {
        assert_msgtype!(Self,msgtype);
        Ok(Self)
    }
}

