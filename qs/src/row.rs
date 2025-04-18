use bytes::{Buf, Bytes};

use crate::{
    ext::BytesExt,
    postgres::backend::{DataRow, RowDescription},
};

pub struct RowDecoder {
    field_len: u16,
    read: u16,
    body: Bytes,
}

impl RowDecoder {
    pub fn new(rowdesc: RowDescription) -> Self {
        Self {
            field_len: rowdesc.field_len,
            read: 0,
            body: rowdesc.body,
        }
    }
}

impl Iterator for RowDecoder {
    type Item = RowInfo;

    fn next(&mut self) -> Option<Self::Item> {
        if self.read == self.field_len {
            return None;
        }
        self.read += 1;
        Some(RowInfo::new(&mut self.body))
    }
}

#[derive(Debug)]
pub struct RowInfo {
    /// The field name.
    pub field_name: String,
    // If the field can be identified as a column of a specific table,
    // the object ID of the table; otherwise zero
    pub table_oid: u32,
    // If the field can be identified as a column of a specific table,
    // the attribute number of the column; otherwise zero.
    pub attribute_len: u16,
    // The object ID of the field's data type.
    pub data_type: u32,
    // The data type size (see pg_type.typlen).
    // Note that negative values denote variable-width types.
    pub data_type_size: i16,
    // The type modifier (see pg_attribute.atttypmod).
    // The meaning of the modifier is type-specific.
    pub type_modifier: i32,
    // The format code being used for the field.
    // Currently will be zero (text) or one (binary).
    // In a RowDescription returned from the statement variant of Describe,
    // the format code is not yet known and will always be zero.
    pub format_code: u16,
}

impl RowInfo {
    pub(crate) fn new(body: &mut Bytes) -> Self {
        Self {
            field_name: body.get_nul_string(),
            table_oid: body.get_u32(),
            attribute_len: body.get_u16(),
            data_type: body.get_u32(),
            data_type_size: body.get_i16(),
            type_modifier: body.get_i32(),
            format_code: body.get_u16(),
        }
    }
}

// NOTE: ---

/// an unencoded row
#[derive(Debug)]
pub struct RowBuffer {
    /// expected column length
    column_len: u16,
    /// already read column
    read: u16,
    /// raw buffer
    bytes: Bytes,
}

impl RowBuffer {
    pub(crate) fn new(datarow: DataRow) -> Self {
        Self {
            column_len: datarow.column_len,
            bytes: datarow.body,
            read: 0,
        }
    }
}

impl Iterator for RowBuffer {
    type Item = Bytes;

    fn next(&mut self) -> Option<Self::Item> {
        if self.read == self.column_len {
            return None
        }

        // The length of the column value, in bytes (this count does not include itself).
        // Can be zero. As a special case, -1 indicates a NULL column value.
        // No value bytes follow in the NULL case.
        let len = self.bytes.get_i32();

        // The value of the column, in the format indicated by the associated format code.
        // n is the above length.
        let data = match len {
            -1 => Bytes::from_static(b"NULL") ,
            len => self.bytes.split_to(len as _) ,
        };

        self.read += 1;

        Some(data)
    }
}
