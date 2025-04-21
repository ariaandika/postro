use bytes::{Buf, Bytes};

use crate::{
    common::InlineVec,
    ext::BytesExt,
    postgres::{Oid, backend::RowDescription},
};

#[derive(Debug)]
pub struct ColumnInfo {
    /// The field name.
    pub field_name: String,
    // // If the field can be identified as a column of a specific table,
    // // the object ID of the table; otherwise zero
    // pub table_oid: u32,
    // // If the field can be identified as a column of a specific table,
    // // the attribute number of the column; otherwise zero.
    // pub attribute_len: u16,
    // The object ID of the field's data type.
    pub data_type: u32,
    // // The data type size (see pg_type.typlen).
    // // Note that negative values denote variable-width types.
    // pub data_type_size: i16,
    // // The type modifier (see pg_attribute.atttypmod).
    // // The meaning of the modifier is type-specific.
    // pub type_modifier: i32,
    // // The format code being used for the field.
    // // Currently will be zero (text) or one (binary).
    // // In a RowDescription returned from the statement variant of Describe,
    // // the format code is not yet known and will always be zero.
    // pub format_code: u16,
}

impl ColumnInfo {
    pub(crate) fn new(body: &mut Bytes) -> Self {
        let field_name = body.get_nul_string();
        let _table_oid = body.advance(size_of::<u32>());
        let _attribute_len = body.advance(size_of::<u16>());
        let data_type = body.get_u32();
        let _data_type_size = body.advance(size_of::<i16>());
        let _type_modifier = body.advance(size_of::<i32>());
        let _format_code = body.advance(size_of::<u16>());
        Self {
            field_name,
            // table_oid,
            // attribute_len,
            data_type,
            // data_type_size,
            // type_modifier,
            // format_code,
        }
    }

    pub(crate) fn decode_multi(mut rd: RowDescription) -> InlineVec<Self, 8> {
        let mut cols = InlineVec::with_capacity(rd.field_len as _);
        for _ in 0..rd.field_len {
            cols.push(Self::new(&mut rd.body));
        }
        cols
    }

    pub(crate) fn decode_multi_vec(mut rd: RowDescription) -> Vec<Self> {
        let mut cols = Vec::with_capacity(rd.field_len as _);
        for _ in 0..rd.field_len {
            cols.push(Self::new(&mut rd.body));
        }
        cols
    }
}

#[derive(Debug)]
pub struct Column {
    pub(crate) oid: Oid,
    pub(crate) value: Bytes,
}

impl Column {
    pub(crate) fn new(col: &ColumnInfo, value: Bytes) -> Self {
        Self {
            oid: col.data_type,
            value,
        }
    }
}

pub trait Index: Sized {
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

