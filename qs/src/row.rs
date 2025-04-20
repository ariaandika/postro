use bytes::{Buf, Bytes};

use crate::{
    Error, Result,
    column::{Column, ColumnInfo, Index},
    decode::Decode,
    postgres::backend::{DataRow, RowDescription},
};

pub trait FromRow: Sized {
    fn from_row(row: Row) -> Result<Self>;
}

impl FromRow for () {
    fn from_row(_: Row) -> Result<Self> {
        Ok(())
    }
}

macro_rules! from_row_tuple {
    ($($t:ident $i:literal),*) => {
        impl<$($t),*> FromRow for ($($t),*,)
        where
            $($t: Decode),*
        {
            fn from_row(row: Row) -> Result<Self> {
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

pub(crate) fn decode_row_desc(mut rd: RowDescription) -> Vec<ColumnInfo> {
    let mut cols = Vec::with_capacity(rd.field_len as _);
    for _ in 0..rd.field_len {
        cols.push(ColumnInfo::new(&mut rd.body));
    }
    cols
}

pub(crate) fn decode_row_data(mut dr: DataRow) -> Vec<Bytes> {
    let mut rows = Vec::with_capacity(dr.column_len as _);
    for _ in 0..dr.column_len {
        let len = dr.body.get_u32();
        rows.push(dr.body.split_to(len as _));
    }
    rows
}

#[derive(Debug)]
pub struct Row<'a> {
    cols: &'a mut Vec<ColumnInfo>,
    values: Vec<Bytes>,
}

impl<'a> Row<'a> {
    pub(crate) fn new(cols: &'a mut Vec<ColumnInfo>, dr: DataRow) -> Self {
        Self { cols, values: decode_row_data(dr) }
    }

    pub fn try_decode<D: Decode, I: Index>(&self, idx: I) -> Result<D> {
        let Some(idx) = idx.position(self.cols.as_slice()) else {
            return Err(Error::ColumnIndexOutOfBounds);
        };
        D::decode(Column::new(&self.cols[idx], self.values[idx].clone()))
    }
}


