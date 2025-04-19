use bytes::{BufMut, Bytes};

/// Integer signess in postgres docs is awful.
pub trait UsizeExt {
    /// length is usize in rust, while postgres want i32,
    /// this will panic when overflow instead of wrapping
    fn to_i32(self) -> i32;
    /// length is usize in rust, while sometime postgres want u32,
    /// this will panic when overflow instead of wrapping
    fn to_u32(self) -> u32;
    /// length is usize in rust, while sometime postgres want u16,
    /// this will panic when overflow instead of wrapping
    fn to_u16(self) -> u16;
}

impl UsizeExt for usize {
    fn to_i32(self) -> i32 {
        self.try_into().expect("message size too large for protocol: {err}")
    }

    fn to_u32(self) -> u32 {
        self.try_into().expect("message size too large for protocol: {err}")
    }

    fn to_u16(self) -> u16 {
        self.try_into().expect("message size too large for protocol: {err}")
    }
}

pub trait StrExt {
    /// postgres String must be nul terminated
    fn nul_string_len(&self) -> i32;
}

impl StrExt for str {
    fn nul_string_len(&self) -> i32 {
        self.len().to_i32() + 1/* nul */
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
    fn get_nul_string(&mut self) -> String;
}

impl BytesExt for Bytes {
    fn get_nul_string(&mut self) -> String {
        let end = self
            .iter()
            .position(|e| matches!(e, b'\0'))
            .expect("Postgres string did not nul terminated");
        let string = self.split_to(end).into();
        // nul
        bytes::Buf::advance(self, 1);
        String::from_utf8(string).expect("Postgres did not return UTF-8")
    }
}
