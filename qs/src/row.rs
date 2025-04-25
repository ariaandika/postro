//! Postgres row operation.
use bytes::{Buf, Bytes};

use crate::{
    column::{Column, ColumnInfo, Index},
    decode::{Decode, DecodeError},
    postgres::backend::DataRow,
};

/// Type that can be constructed from a row.
pub trait FromRow: Sized {
    /// Construct self from row.
    fn from_row(row: Row) -> Result<Self, DecodeError>;
}

impl FromRow for () {
    fn from_row(_: Row) -> Result<Self, DecodeError> {
        Ok(())
    }
}

macro_rules! from_row_tuple {
    ($($t:ident $i:literal),*) => {
        impl<$($t),*> FromRow for ($($t),*,)
        where
            $($t: Decode),*
        {
            fn from_row(row: Row) -> Result<Self, DecodeError> {
                Ok((
                    $(row.try_decode($i)?),*,
                ))
            }
        }
    };
}

from_row_tuple!(T0 0);
from_row_tuple!(T0 0, T1 1);
from_row_tuple!(T0 0, T1 1, T2 2);
from_row_tuple!(T0 0, T1 1, T2 2, T3 3);

fn decode_row_data(mut dr: DataRow) -> Vec<Bytes> {
    let mut rows = Vec::with_capacity(dr.column_len as _);
    for _ in 0..dr.column_len {
        let len = dr.body.get_u32();
        rows.push(dr.body.split_to(len as _));
    }
    rows
}

/// Postgres row.
#[derive(Debug)]
pub struct Row<'a> {
    cols: &'a mut [ColumnInfo],
    values: Vec<Bytes>,
}

impl<'a> Row<'a> {
    pub(crate) fn new(cols: &'a mut [ColumnInfo], dr: DataRow) -> Self {
        Self { cols, values: decode_row_data(dr) }
    }

    /// Try decode specified column.
    pub fn try_decode<D: Decode, I: Index>(&self, idx: I) -> Result<D, DecodeError> {
        let Some(idx) = idx.position(self.cols) else {
            return Err(DecodeError::IndexOutOfBound);
        };
        D::decode(Column::new(&self.cols[idx], self.values[idx].clone()))
    }
}


