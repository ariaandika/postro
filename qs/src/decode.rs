use crate::{column::Column, postgres::PgType, Error, Result};


pub trait Decode: Sized {
    fn decode(col: Column) -> Result<Self>;
}

impl Decode for i32 {
    fn decode(col: Column) -> Result<Self> {
        if col.oid != i32::OID {
            return Err(Error::MissmatchDataType);
        }
        let mut be = [0u8;4];
        be.copy_from_slice(&col.value[..4]);
        Ok(i32::from_be_bytes(be))
    }
}

impl Decode for String {
    fn decode(col: Column) -> Result<Self> {
        if col.oid != String::OID {
            return Err(Error::MissmatchDataType);
        }
        Ok(String::from_utf8(col.value.into()).unwrap())
    }
}

