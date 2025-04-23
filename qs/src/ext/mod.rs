use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::common::ByteStr;

/// Integer signess in postgres docs is awful.
pub trait UsizeExt {
    /// length is usize in rust, while sometime postgres want u32,
    /// this will panic when overflow instead of wrapping
    fn to_u32(self) -> u32;
    /// length is usize in rust, while sometime postgres want u16,
    /// this will panic when overflow instead of wrapping
    fn to_u16(self) -> u16;
}

impl UsizeExt for usize {
    fn to_u32(self) -> u32 {
        self.try_into().expect("message size too large for protocol: {err}")
    }

    fn to_u16(self) -> u16 {
        self.try_into().expect("message size too large for protocol: {err}")
    }
}

pub trait StrExt {
    /// postgres String must be nul terminated
    fn nul_string_len(&self) -> u32;
}

impl StrExt for str {
    fn nul_string_len(&self) -> u32 {
        self.len().to_u32() + 1/* nul */
    }
}

pub trait BufMutExt {
    /// postgres String must be nul terminated
    fn put_nul_string(&mut self, string: &str);
}

impl<B: BufMut> BufMutExt for B {
    fn put_nul_string(&mut self, string: &str) {
        self.put(string.as_bytes());
        self.put_u8(b'\0');
    }
}

pub trait BytesExt {
    /// Using [`ByteStr`] avoid allocating [`Vec`] as it required for [`String::from_utf8`]
    fn get_nul_bytestr(&mut self) -> Result<ByteStr, std::str::Utf8Error>;
}

impl BytesExt for Bytes {
    fn get_nul_bytestr(&mut self) -> Result<ByteStr, std::str::Utf8Error> {
        let end = self
            .iter()
            .position(|e| matches!(e, b'\0'))
            .expect("Postgres string did not nul terminated");
        let me = self.split_to(end);
        Buf::advance(self, 1); // nul
        ByteStr::from_utf8(me)
    }
}

impl BytesExt for BytesMut {
    fn get_nul_bytestr(&mut self) -> Result<ByteStr, std::str::Utf8Error> {
        let end = self
            .iter()
            .position(|e| matches!(e, b'\0'))
            .expect("Postgres string did not nul terminated");
        let me = self.split_to(end);
        Buf::advance(self, 1); // nul
        ByteStr::from_utf8(me.freeze())
    }
}

pub trait BindParams: Buf {
    /// The length of the parameter value, in bytes (this count does not include itself).
    ///
    /// Can be zero. As a special case, -1 indicates a NULL parameter value.
    /// No value bytes follow in the NULL case.
    fn size(&self) -> i32;
}


pub trait FmtExt {
    fn lossy(&self) -> LossyFmt<'_>;
}

impl FmtExt for [u8] {
    fn lossy(&self) -> LossyFmt<'_> {
        LossyFmt(self)
    }
}

pub struct LossyFmt<'a>(pub &'a [u8]);

impl std::fmt::Display for LossyFmt<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for &b in self.0 {
            if b.is_ascii_graphic() || b.is_ascii_whitespace() {
                write!(f, "{}", b as char)?;
            } else {
                write!(f, "\\x{b:x}")?;
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for LossyFmt<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "b\"{self}\"")
    }
}

