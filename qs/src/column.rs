use bytes::{Buf, Bytes};

use crate::{
    common::ByteStr,
    ext::BytesExt,
    postgres::{backend::RowDescription, Oid},
};

#[derive(Debug)]
pub struct ColumnInfo {
    /// The field name.
    pub field_name: ByteStr,
    // The object ID of the field's data type.
    pub data_type: u32,
}

impl ColumnInfo {
    pub(crate) fn decode(body: &mut Bytes) -> Result<Self, std::str::Utf8Error> {
        let field_name = body.get_nul_bytestr()?;
        body.advance(size_of::<u32>()); // table_oid
        body.advance(size_of::<u16>()); // attribute_len
        let data_type = body.get_u32();
        body.advance(size_of::<i16>()); // data_type_size
        body.advance(size_of::<i32>()); // type_modifier
        body.advance(size_of::<u16>()); // format_code
        Ok(Self {
            field_name,
            data_type,
        })
    }

    pub(crate) fn decode_multi_vec(mut rd: RowDescription) -> Result<Vec<ColumnInfo>, std::str::Utf8Error> {
        let mut cols = Vec::with_capacity(rd.field_len as _);
        for _ in 0..rd.field_len {
            cols.push(Self::decode(&mut rd.body)?);
        }
        Ok(cols)
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

