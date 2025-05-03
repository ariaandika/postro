use bytes::{Buf, Bytes};

const INLINE_LEN: usize = 15;

pub(crate) enum ValueRef<'a> {
    Slice(&'a [u8]),
    Inline {
        offset: usize,
        value: [u8;INLINE_LEN],
    },
    Bytes(Bytes)
}

impl ValueRef<'_> {
    pub fn inline(slice: &[u8]) -> ValueRef<'static> {
        let len = slice.len();
        assert!(len < INLINE_LEN, "inline slice is too large");
        let mut value = [0u8;INLINE_LEN];
        value[INLINE_LEN - len..].copy_from_slice(slice);
        ValueRef::Inline { offset: INLINE_LEN - len, value }
    }

    pub fn len(&self) -> usize {
        match self {
            ValueRef::Slice(items) => items.len(),
            ValueRef::Inline { offset, .. } => INLINE_LEN - offset,
            ValueRef::Bytes(bytes) => bytes.len(),
        }
    }
}

impl Buf for ValueRef<'_> {
    fn remaining(&self) -> usize {
        match self {
            ValueRef::Slice(items) => Buf::remaining(items),
            ValueRef::Inline { offset, .. } => INLINE_LEN - offset,
            ValueRef::Bytes(bytes) => Buf::remaining(bytes),
        }
    }

    fn chunk(&self) -> &[u8] {
        match self {
            ValueRef::Slice(items) => Buf::chunk(items),
            ValueRef::Inline { offset, value } => &value[*offset..],
            ValueRef::Bytes(bytes) => Buf::chunk(bytes),
        }
    }

    fn advance(&mut self, cnt: usize) {
        match self {
            ValueRef::Slice(items) => Buf::advance(items, cnt),
            ValueRef::Inline { offset, .. } => *offset += cnt,
            ValueRef::Bytes(bytes) => Buf::advance(bytes, cnt),
        }
    }
}

impl std::fmt::Debug for ValueRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use crate::ext::FmtExt;
        self.chunk().lossy().fmt(f)
    }
}

