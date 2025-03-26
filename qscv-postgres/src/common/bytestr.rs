use bytes::Bytes;


/// [`Bytes`] based str
pub struct ByteStr {
    bytes: Bytes,
}

impl ByteStr {
    pub fn copy_from(string: &str) -> Self {
        Self { bytes: Bytes::copy_from_slice(string.as_bytes()) }
    }
}

impl std::ops::Deref for ByteStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        // SAFETY: input is always string and immutable
        unsafe { std::str::from_utf8_unchecked(&self.bytes) }
    }
}


