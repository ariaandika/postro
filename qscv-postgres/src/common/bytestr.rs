use bytes::Bytes;

/// A cheaply cloneable and sliceable str.
pub struct ByteStr {
    bytes: Bytes,
}

impl ByteStr {
    /// Creates `ByteStr` instance from str slice, by copying it.
    pub fn copy_from_str(string: &str) -> Self {
        Self { bytes: Bytes::copy_from_slice(string.as_bytes()) }
    }

    /// Creates a new `ByteStr` from a static str.
    ///
    /// The returned `ByteStr` will point directly to the static str. There is
    /// no allocating or copying.
    pub const fn from_static(string: &'static str) -> Self {
        Self { bytes: Bytes::from_static(string.as_bytes()) }
    }

    /// Returns a slice str of self that is equivalent to the given `subset`.
    ///
    /// This operation is `O(1)`.
    ///
    /// # Panics
    ///
    /// Requires that the given `sub` slice str is in fact contained within the
    /// `ByteStr` buffer; otherwise this function will panic.
    ///
    /// see also [`Bytes::slice_ref`]
    pub fn slice_ref(&self, subset: &str) -> Self {
        Self { bytes: Bytes::slice_ref(&self.bytes, subset.as_bytes()) }
    }

    /// return the internal str
    pub fn as_ref(&self) -> &str {
        // SAFETY: input is a string and immutable
        unsafe { std::str::from_utf8_unchecked(&self.bytes) }
    }

    /// clone the underlying bytes
    #[allow(unused)]
    pub fn bytes(&self) -> Bytes {
        self.bytes.clone()
    }
}

impl std::ops::Deref for ByteStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl Clone for ByteStr {
    fn clone(&self) -> Self {
        Self { bytes: Bytes::clone(&self.bytes) }
    }
}

impl Default for ByteStr {
    fn default() -> Self {
        Self { bytes: Bytes::new() }
    }
}

impl std::fmt::Display for ByteStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <str as std::fmt::Display>::fmt(self, f)
    }
}

impl std::fmt::Debug for ByteStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.bytes, f)
    }
}

impl PartialEq for ByteStr {
    fn eq(&self, other: &Self) -> bool {
        str::eq(self.as_ref(), other.as_ref())
    }
}

impl PartialEq<str> for ByteStr {
    fn eq(&self, other: &str) -> bool {
        str::eq(self, other)
    }
}

impl PartialEq<&str> for ByteStr {
    fn eq(&self, other: &&str) -> bool {
        str::eq(self, *other)
    }
}

impl<B> From<B> for ByteStr where B: Into<Bytes> {
    fn from(value: B) -> Self {
        Self { bytes: value.into() }
    }
}

