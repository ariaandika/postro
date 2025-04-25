//! Row value decoding.
use crate::{column::Column, postgres::PgType};

mod error;

pub use error::DecodeError;


/// Type that can be decoded from column value.
pub trait Decode: Sized {
    /// Construct self from a column.
    fn decode(col: Column) -> Result<Self, DecodeError>;
}

impl Decode for () {
    fn decode(_: Column) -> Result<Self, DecodeError> {
        Ok(())
    }
}

impl Decode for i32 {
    fn decode(col: Column) -> Result<Self, DecodeError> {
        if col.oid() != i32::OID {
            return Err(DecodeError::OidMissmatch);
        }
        let mut be = [0u8;size_of::<i32>()];
        be.copy_from_slice(&col.as_slice()[..size_of::<i32>()]);
        Ok(i32::from_be_bytes(be))
    }
}

impl Decode for String {
    fn decode(col: Column) -> Result<Self, DecodeError> {
        if col.oid() != String::OID {
            return Err(DecodeError::OidMissmatch);
        }
        Ok(String::from_utf8(col.into_value().into())?)
    }
}

