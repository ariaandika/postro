use bytes::Bytes;

#[derive(Debug)]
pub enum ValueRef<'a> {
    Null,
    Bool(bool),
    Number(i32),
    Text(&'a str),
    String(String),
    Slice(&'a [u8]),
    Bytes(Bytes),
}

impl From<()> for ValueRef<'static> {
    fn from(_: ()) -> Self {
        Self::Null
    }
}

impl From<i32> for ValueRef<'static> {
    fn from(value: i32) -> Self {
        Self::Number(value)
    }
}

impl From<bool> for ValueRef<'static> {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl<'a> From<&'a str> for ValueRef<'a> {
    fn from(value: &'a str) -> Self {
        Self::Text(value)
    }
}

impl From<String> for ValueRef<'static> {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl<'a> From<&'a String> for ValueRef<'a> {
    fn from(value: &'a String) -> Self {
        Self::Text(value.as_str())
    }
}

impl<'a> From<&'a [u8]> for ValueRef<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self::Slice(value)
    }
}

impl From<Vec<u8>> for ValueRef<'static> {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(value.into())
    }
}

impl<'a> From<&'a Vec<u8>> for ValueRef<'a> {
    fn from(value: &'a Vec<u8>) -> Self {
        Self::Slice(value.as_slice())
    }
}

impl From<Bytes> for ValueRef<'static> {
    fn from(value: Bytes) -> Self {
        Self::Bytes(value)
    }
}

