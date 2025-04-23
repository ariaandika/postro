use crate::{Error, Result, column::Column, postgres::PgType};

/// Type that can be decoded from column value.
pub trait Decode: Sized {
    /// Construct self from a column.
    fn decode(col: Column) -> Result<Self>;
}

impl Decode for () {
    fn decode(_: Column) -> Result<Self> {
        Ok(())
    }
}

impl Decode for i32 {
    fn decode(col: Column) -> Result<Self> {
        if col.oid() != i32::OID {
            return Err(Error::MissmatchDataType);
        }
        let mut be = [0u8;size_of::<i32>()];
        be.copy_from_slice(&col.as_slice()[..size_of::<i32>()]);
        Ok(i32::from_be_bytes(be))
    }
}

impl Decode for String {
    fn decode(col: Column) -> Result<Self> {
        if col.oid() != String::OID {
            return Err(Error::MissmatchDataType);
        }
        Ok(String::from_utf8(col.into_value().into()).map_err(|e|e.utf8_error())?)
    }
}

