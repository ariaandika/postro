use bytes::Bytes;

/// A cheaply cloneable and sliceable str.
///
/// Using `ByteStr` helps prevent allocating vec as it required by [`String::from_utf8`].
#[derive(Clone)]
pub struct ByteStr {
    bytes: Bytes,
}

impl ByteStr {
    /// Create new empty [`ByteStr`].
    pub const fn new() -> ByteStr {
        Self { bytes: Bytes::new() }
    }

    /// Converts a [`Bytes`] to a [`ByteStr`].
    pub fn from_utf8(bytes: Bytes) -> Result<Self, std::str::Utf8Error> {
        std::str::from_utf8(&bytes)?;
        Ok(Self { bytes })
    }

    /// Creates [`ByteStr`] instance from str slice, by copying it.
    pub fn copy_from_str(string: &str) -> Self {
        Self { bytes: Bytes::copy_from_slice(string.as_bytes()) }
    }

    /// Creates a new [`ByteStr`] from a static str.
    ///
    /// The returned `ByteStr` will point directly to the static str. There is
    /// no allocating or copying.
    pub const fn from_static(string: &'static str) -> Self {
        Self { bytes: Bytes::from_static(string.as_bytes()) }
    }

    /// Extracts a string slice containing the entire `ByteStr`.
    pub fn as_str(&self) -> &str {
        // SAFETY: input is a string and immutable
        unsafe { std::str::from_utf8_unchecked(&self.bytes) }
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

    /// Consume `ByteStr` into [`String`].
    pub fn into_string(self) -> String {
        // SAFETY: input is a string and immutable
        unsafe { String::from_utf8_unchecked(Vec::from(self.bytes)) }
    }

    /// Converts a `ByteStr` into a [`Bytes`].
    ///
    /// This consumes the `ByteStr`, so we do not need to copy its contents.
    pub fn into_bytes(self) -> Bytes {
        self.bytes
    }
}

impl AsRef<str> for ByteStr {
    /// return the internal str
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::ops::Deref for ByteStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Default for ByteStr {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ByteStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_str(), f)
    }
}

impl std::fmt::Debug for ByteStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl PartialEq for ByteStr {
    fn eq(&self, other: &Self) -> bool {
        str::eq(self.as_str(), other.as_str())
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

impl From<&'static str> for ByteStr {
    fn from(value: &'static str) -> Self {
        Self::from_static(value)
    }
}

impl From<std::borrow::Cow<'static,str>> for ByteStr {
    fn from(value: std::borrow::Cow<'static,str>) -> Self {
        match value {
            std::borrow::Cow::Borrowed(s) => Self::from(s),
            std::borrow::Cow::Owned(s) => Self::from(s),
        }
    }
}

impl From<String> for ByteStr {
    fn from(value: String) -> Self {
        Self { bytes: Bytes::from(value.into_bytes()) }
    }
}

