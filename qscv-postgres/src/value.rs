use super::types::Oid;


#[derive(Debug)]
pub enum ValueRef<'a> {
    Null,
    Text(&'a str),
    Bytes(&'a [u8]),
    Number(i32),
    Bool(bool),
}

impl<'a> ValueRef<'a> {
    pub fn oid(&self) -> Oid {
        todo!()
    }
}

impl From<i32> for ValueRef<'static> {
    fn from(value: i32) -> Self {
        Self::Number(value)
    }
}

impl<'a> From<&'a str> for ValueRef<'a> {
    fn from(value: &'a str) -> Self {
        Self::Text(value)
    }
}


