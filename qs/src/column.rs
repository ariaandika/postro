use bytes::{Buf, Bytes};

use crate::{
    common::ByteStr,
    ext::BytesExt,
    postgres::{Oid, backend::RowDescription},
};

#[derive(Debug)]
pub struct ColumnInfo {
    /// The field name.
    field_name: ByteStr,
    // The object ID of the field's data type.
    oid: Oid,
}

impl ColumnInfo {
    pub(crate) fn decode(body: &mut Bytes) -> Result<Self, std::str::Utf8Error> {
        // <https://www.postgresql.org/docs/current/protocol-message-formats.html#PROTOCOL-MESSAGE-FORMATS-ROWDESCRIPTION>
        let field_name = body.get_nul_bytestr()?;
        body.advance(size_of::<u32>()); // table_oid
        body.advance(size_of::<u16>()); // attribute_len
        let oid = body.get_u32();
        body.advance(size_of::<i16>()); // data_type_size
        body.advance(size_of::<i32>()); // type_modifier
        body.advance(size_of::<u16>()); // format_code
        Ok(Self {
            field_name,
            oid,
        })
    }

    pub(crate) fn decode_multi_vec(mut rd: RowDescription) -> Result<Vec<ColumnInfo>, std::str::Utf8Error> {
        let mut cols = Vec::with_capacity(rd.field_len as _);
        for _ in 0..rd.field_len {
            cols.push(Self::decode(&mut rd.body)?);
        }
        Ok(cols)
    }

    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    pub fn oid(&self) -> Oid {
        self.oid
    }
}

/// Column information.
#[derive(Debug)]
pub struct Column {
    oid: Oid,
    value: Bytes,
}

impl Column {
    pub(crate) fn new(col: &ColumnInfo, value: Bytes) -> Self {
        Self {
            oid: col.oid,
            value,
        }
    }

    /// Get the column [`Oid`].
    pub fn oid(&self) -> Oid {
        self.oid
    }

    /// Get the column value as slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.value
    }

    /// Consume the column value as [`Bytes`].
    pub fn into_value(self) -> Bytes {
        self.value
    }
}

/// Type that can be used for indexing column.
///
/// implemented for
/// - `&str`
/// - `usize`
pub trait Index: Sized {
    /// Returns the column index.
    fn position(self, cols: &[ColumnInfo]) -> Option<usize>;
}

impl Index for usize {
    fn position(self, cols: &[ColumnInfo]) -> Option<usize> {
        cols.get(self).is_some().then_some(self)
    }
}

impl Index for &str {
    fn position(self, cols: &[ColumnInfo]) -> Option<usize> {
        cols.iter().position(|e|e.field_name == self)
    }
}

