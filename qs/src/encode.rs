use crate::types::{AsPgType, Oid};
use crate::value::ValueRef;

/// postgres encoded value
#[derive(Debug)]
pub struct Encoded<'q> {
    value: ValueRef<'q>,
    oid: Oid,
}

impl Default for Encoded<'_> {
    fn default() -> Self {
        Self {
            value: ValueRef::Null,
            oid: <()>::PG_TYPE.oid(),
        }
    }
}

impl<'q> Encoded<'q> {
    pub fn new(value: ValueRef<'q>, oid: Oid) -> Self {
        Self { value, oid }
    }

    pub fn value(&self) -> &ValueRef<'q> {
        &self.value
    }

    pub fn oid(&self) -> i32 {
        self.oid
    }
}

/// value that can be encoded to be bound to sql parameter
pub trait Encode<'q> {
    fn encode(self) -> Encoded<'q>;
}

impl Encode<'static> for bool {
    fn encode(self) -> Encoded<'static> {
        Encoded {
            value: ValueRef::Bool(self),
            oid: bool::PG_TYPE.oid(),
        }
    }
}

impl Encode<'static> for i32 {
    fn encode(self) -> Encoded<'static> {
        Encoded {
            value: self.into(),
            oid: i32::PG_TYPE.oid(),
        }
    }
}

impl<'q> Encode<'q> for &'q str {
    fn encode(self) -> Encoded<'q> {
        Encoded {
            value: self.into(),
            oid: str::PG_TYPE.oid(),
        }
    }
}

impl Encode<'static> for String {
    fn encode(self) -> Encoded<'static> {
        Encoded {
            value: self.into(),
            oid: String::PG_TYPE.oid(),
        }
    }
}

impl<'q> Encode<'q> for &'q String {
    fn encode(self) -> Encoded<'q> {
        Encoded {
            value: self.into(),
            oid: String::PG_TYPE.oid(),
        }
    }
}

