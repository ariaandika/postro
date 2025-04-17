use bytes::{Buf, Bytes};

use super::ext::BytesExt;

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
            field_name: body
                .get_nul_string()
                .expect("invalid RowDescription from postgres"),
            table_oid: body.get_u32(),
            attribute_len: body.get_u16(),
            data_type: body.get_u32(),
            data_type_size: body.get_i16(),
            type_modifier: body.get_i32(),
            format_code: body.get_u16(),
        }
    }
}

