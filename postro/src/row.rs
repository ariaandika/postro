//! Postgres row operation.
//!
//! - [`Row`]
//! - [`Column`]
//! - [`FromRow`]
//! - [`Decode`]
//!
//! - [`Index`]
//! - [`DecodeError`]
use bytes::{Buf, Bytes};
use std::{borrow::Cow, fmt, str::Utf8Error, string::FromUtf8Error};

use crate::{
    common::{ByteStr, unit_error},
    ext::{BytesExt, FmtExt},
    postgres::{Oid, PgType},
};

// <https://www.postgresql.org/docs/current/protocol-message-formats.html#PROTOCOL-MESSAGE-FORMATS-ROWDESCRIPTION>
// table_oid
// attribute_len
// oid
// data_type_size
// type_modifier
// format_code
const SUFFIX: usize = size_of::<u32>()
    + size_of::<u16>()
    + size_of::<u32>()
    + size_of::<i16>()
    + size_of::<i32>()
    + size_of::<u16>();

const OID_OFFSET: usize = size_of::<u32>() + size_of::<u16>();

/// Postgres row.
pub struct Row {
    field_len: u16,
    body: Bytes,
    values: Bytes,
}

impl Row {
    /// `RowDescription` message
    pub(crate) fn new(mut bytes: Bytes) -> Self {
        Self {
            field_len: bytes.get_u16(),
            body: bytes,
            values: Bytes::new(),
        }
    }

    /// `DataRow` message
    pub(crate) fn inner_clone(&self, mut bytes: Bytes) -> Row {
        assert_eq!(
            self.field_len, bytes.get_u16(),
            "RowDescription len missmatch with DataRow len"
        );
        Self {
            field_len: self.field_len,
            body: self.body.clone(),
            values: bytes,
        }
    }

    /// Returns `true` if row contains no columns.
    pub const fn is_empty(&self) -> bool {
        self.field_len == 0
    }

    /// Returns the number of fields/column.
    pub const fn len(&self) -> u16 {
        self.field_len
    }

    /// Try get and decode column.
    pub fn try_get<I: Index, R: Decode>(&self, idx: I) -> Result<R, DecodeError> {
        let (offset,nul,nth) = idx.position(&self.body, self.field_len)?;

        let name = ByteStr::from_utf8(self.body.slice(offset..nul))?;

        let mut i = 0;
        let mut values = self.values.clone();
        let value = loop {
            let len = values.get_i32();
            let value = match len {
                -1 => None,
                _ => Some(values.split_to(len as _)),
            };
            if i == nth {
                break value;
            }
            i += 1;
        };

        R::decode(Column::new(name, &self.body[nul + 1..], value))
    }

    /// Try decode type using [`FromRow`] implementation.
    pub fn decode<D: FromRow>(self) -> Result<D, DecodeError> {
        D::from_row(self)
    }
}

impl IntoIterator for Row {
    type Item = Result<Column, DecodeError>;

    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            field_len: self.field_len,
            body: self.body,
            values: self.values,
            iter_n: 0,
        }
    }
}

/// [`IntoIterator`] implementation from [`Row`].
#[derive(Debug)]
pub struct IntoIter {
    field_len: u16,
    body: Bytes,
    values: Bytes,

    iter_n: u16,
}

impl IntoIter {
    /// Same as [`Iterator::next`] but returns [`Result`] instead.
    pub fn try_next(&mut self) -> Result<Column, DecodeError> {
        match self.next() {
            Some(ok) => ok,
            None => Err(DecodeError::IndexOutOfBounds(self.iter_n as _)),
        }
    }
}

impl Iterator for IntoIter {
    type Item = Result<Column, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter_n == self.field_len {
            return None
        }

        let field_name = match self.body.get_nul_bytestr() {
            Ok(ok) => ok,
            Err(err) => {
                self.iter_n = self.field_len;
                return Some(Err(err.into()))
            },
        };
        let column = self.body.split_to(SUFFIX);
        let len = self.values.get_i32();
        let value = match len {
            -1 => None,
            _ => Some(self.values.split_to(len as _)),
        };
        self.iter_n += 1;

        Some(Ok(Column::new(field_name, &column, value)))
    }
}

impl fmt::Debug for Row {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_map();
        let mut b = self.body.clone();
        let mut v = self.values.clone();
        for _ in 0..self.field_len {
            let Ok(key) = b.get_nul_bytestr() else { break };
            b.advance(SUFFIX);
            let len = v.get_i32();
            dbg.key(&key);
            match len {
                -1 => dbg.value(&format_args!("NULL")),
                len => dbg.value(&v.split_to(len as _).lossy()),
            };
        }
        dbg.finish()
    }
}

/// Postgres column.
#[derive(Debug, Clone)]
pub struct Column {
    oid: Oid,
    value: Option<Bytes>,
    name: ByteStr,
}

impl Column {
    /// `body` is start of data **after** field name
    fn new(name: ByteStr, body: &[u8], value: Option<Bytes>) -> Self {
        Self {
            name,
            oid: (&mut &body[OID_OFFSET..]).get_u32(),
            value
        }
    }

    /// Returns column [`Oid`].
    pub const fn oid(&self) -> Oid {
        self.oid
    }

    /// Returns column name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return `true` if value is NULL.
    pub const fn is_null(&self) -> bool {
        self.value.is_none()
    }

    /// Extract the inner bytes as slice.
    ///
    /// Returns [`None`] if value is `NULL`.
    pub fn as_slice(&self) -> Option<&[u8]> {
        self.value.as_deref()
    }

    /// Clone the inner [`Bytes`].
    ///
    /// Returns [`None`] if value is `NULL`.
    pub fn value(&self) -> Option<Bytes> {
        self.value.as_ref().cloned()
    }

    /// Consume self into the inner [`Bytes`].
    ///
    /// Returns empty [`Bytes`] if value is `NULL`.
    pub fn into_value(self) -> Option<Bytes> {
        self.value
    }

    /// Try consume self into the inner [`Bytes`].
    ///
    /// Return [`DecodeError::Null`] if value is `NULL`.
    pub fn try_into_value(self) -> Result<Bytes, DecodeError> {
        self.value.ok_or(DecodeError::Null)
    }

    /// Try decode type using [`Decode`] implementation.
    pub fn decode<D: Decode>(self) -> Result<D, DecodeError> {
        D::decode(self)
    }
}

/// Query result with its rows affected.
#[derive(Debug)]
pub struct RowResult {
    pub rows_affected: u64,
}

// ===== Traits =====

/// Type that can be constructed from a row.
pub trait FromRow: Sized {
    /// Construct self from row.
    fn from_row(row: Row) -> Result<Self, DecodeError>;
}

impl FromRow for Row {
    fn from_row(row: Row) -> Result<Self, DecodeError> {
        Ok(row)
    }
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
                    $(row.try_get($i)?),*,
                ))
            }
        }
    };
}

from_row_tuple!(T0 0);
from_row_tuple!(T0 0, T1 1);
from_row_tuple!(T0 0, T1 1, T2 2);
from_row_tuple!(T0 0, T1 1, T2 2, T3 3);

/// A type that can be constructed from [`Column`].
pub trait Decode: Sized {
    /// Try decode self from column.
    fn decode(column: Column) -> Result<Self, DecodeError>;
}

impl Decode for Column {
    fn decode(column: Column) -> Result<Self, DecodeError> {
        Ok(column)
    }
}

impl<T: Decode> Decode for Option<T> {
    fn decode(column: Column) -> Result<Self, DecodeError> {
        match column.is_null() {
            true => Ok(None),
            false => column.decode().map(Some),
        }
    }
}

impl Decode for () {
    fn decode(_: Column) -> Result<Self, DecodeError> {
        Ok(())
    }
}

impl Decode for i32 {
    fn decode(col: Column) -> Result<Self, DecodeError> {
        if col.oid() != Self::OID {
            return Err(DecodeError::OidMissmatch);
        }
        let mut be = [0u8;size_of::<Self>()];
        be.copy_from_slice(&col.try_into_value()?[..size_of::<Self>()]);
        Ok(i32::from_be_bytes(be))
    }
}

impl Decode for String {
    fn decode(col: Column) -> Result<Self, DecodeError> {
        if col.oid() != Self::OID {
            return Err(DecodeError::OidMissmatch);
        }
        Ok(String::from_utf8(col.try_into_value().map(Into::into)?)?)
    }
}

/// Type that can be used for indexing column.
pub trait Index: Sized + sealed::Sealed {
    /// Returns (bytes start offset, nul string index, nth column).
    fn position(self, body: &[u8], len: u16) -> Result<(usize,usize,u16), DecodeError>;
}

macro_rules! position {
    (
        $self:pat, $body:ident, $len:ident,
        ($offset:ident,$i_nul:ident,$nth:ident) => $test:expr,
        () => $into:expr
    ) => {
        let mut iter = $body.iter().copied().enumerate();
        let mut $offset = 0;

        for $nth in 0..$len {
            let Some(($i_nul, _)) = iter.find(|(_, e)| matches!(e, b'\0')) else {
                break;
            };

            if $test {
                return Ok(($offset,$i_nul,$nth));
            }

            match iter.$nth(SUFFIX) {
                Some((i,_)) => {
                    $offset = i;
                },
                None => break,
            }
        }

        Err(DecodeError::ColumnNotFound($into))
    };
}

impl Index for usize {
    fn position(self, body: &[u8], len: u16) -> Result<(usize,usize,u16), DecodeError> {
        position! {
            self, body, len,
            (off,i_nul,nth) => self == nth as usize,
            () => String::from(itoa::Buffer::new().format(self)).into()
        }
    }
}

impl Index for &str {
    fn position(self, body: &[u8], len: u16) -> Result<(usize,usize,u16), DecodeError> {
        position! {
            self, body, len,
            (off,i_nul,nth) => self.as_bytes() == &body[off..i_nul],
            () => String::from(self).into()
        }
    }
}

mod sealed {
    pub trait Sealed { }
    impl Sealed for usize { }
    impl Sealed for &str { }
}

unit_error! {
    /// An error when try to [`fetch_one`][crate::query::Query::fetch_one] and not returns any row.
    pub struct RowNotFound("row not found");
}

macro_rules! from {
    (<$ty:ty>$pat:pat => $body:expr) => {
        impl From<$ty> for DecodeError {
            fn from($pat: $ty) -> Self {
                $body
            }
        }
    };
}

/// An error when decoding row value.
pub enum DecodeError {
    /// Postgres return non utf8 string.
    Utf8(Utf8Error),
    /// Column requested not found.
    ColumnNotFound(Cow<'static,str>),
    /// Index requested is out of bounds.
    IndexOutOfBounds(usize),
    /// Oid requested missmatch.
    OidMissmatch,
    /// Row is null.
    Null,
    /// Failed to deserialize using `serde_json`.
    #[cfg(feature = "json")]
    Json(serde_json::error::Error),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("failed to decode value, ")?;
        match self {
            Self::Utf8(e) => write!(f, "{e}"),
            Self::ColumnNotFound(name) => write!(f, "column not found: {name:?}"),
            Self::IndexOutOfBounds(u) => write!(f, "index out of bounds: {u:?}"),
            Self::OidMissmatch => write!(f, "data type missmatch"),
            Self::Null => write!(f, "unexpected NULL value"),
            #[cfg(feature = "json")]
            Self::Json(e) => write!(f, "{e}"),
        }
    }
}

from!(<Utf8Error>e => Self::Utf8(e));
from!(<FromUtf8Error>e => Self::Utf8(e.utf8_error()));
#[cfg(feature = "json")]
from!(<serde_json::error::Error>e => Self::Json(e));

impl std::error::Error for DecodeError { }

impl fmt::Debug for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{self}\"")
    }
}

