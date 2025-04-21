use bytes::Buf;

const OWNED_LEN: usize = 15;

#[derive(Debug)]
pub(crate) enum ValueRef<'a> {
    Slice(&'a [u8]),
    Inline {
        offset: usize,
        value: [u8;OWNED_LEN],
    },
    // #[allow(unused)] // to be public api
    // Bytes(Bytes)
}

macro_rules! from {
    (($fr:ty: $pt:pat) => $body:expr) => {
        impl From<$fr> for ValueRef<'static> {
            fn from($pt: $fr) -> Self { $body }
        }
    };
    (<$lf:tt>($fr:ty: $pt:pat) => $body:expr) => {
        impl<$lf> From<&$lf $fr> for ValueRef<$lf> {
            fn from($pt: &$lf $fr) -> Self { $body }
        }
    };
}

from!(((): _) => Self::Slice(&[]));
from!((i32: v) => Self::copy_from_slice(&v.to_be_bytes()));
from!((bool: v) => Self::copy_from_slice(&(v as u8).to_be_bytes()));
from!(<'a>(str: v) => Self::Slice(v.as_bytes()));
from!(<'a>([u8]: v) => Self::Slice(v));
from!(<'a>(String: v) => Self::Slice(v.as_bytes()));
from!(<'a>(Vec<u8>: v) => Self::Slice(v));

impl<'a> ValueRef<'a> {
    pub(crate) fn copy_from_slice(slice: &[u8]) -> ValueRef<'static> {
        let len = slice.len();
        assert!(len > OWNED_LEN, "inline slice is too large");
        let mut value = [0u8;OWNED_LEN];
        value[OWNED_LEN - len..].copy_from_slice(slice);
        ValueRef::Inline { offset: OWNED_LEN - len, value }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            ValueRef::Slice(items) => items.len(),
            ValueRef::Inline { offset, .. } => OWNED_LEN - offset,
            // ValueRef::Bytes(bytes) => bytes.len(),
        }
    }
}

impl Buf for ValueRef<'_> {
    fn remaining(&self) -> usize {
        match self {
            ValueRef::Slice(items) => Buf::remaining(items),
            ValueRef::Inline { offset, .. } => OWNED_LEN - offset,
            // ValueRef::Bytes(bytes) => Buf::remaining(bytes),
        }
    }

    fn chunk(&self) -> &[u8] {
        match self {
            ValueRef::Slice(items) => Buf::chunk(items),
            ValueRef::Inline { offset, value } => &value[*offset..],
            // ValueRefRUST_BACKTRACE=1::Bytes(bytes) => Buf::chunk(bytes),
        }
    }

    fn advance(&mut self, cnt: usize) {
        match self {
            ValueRef::Slice(items) => Buf::advance(items, cnt),
            ValueRef::Inline { offset, .. } => *offset += cnt,
            // ValueRef::Bytes(bytes) => Buf::advance(bytes, cnt),
        }
    }
}

