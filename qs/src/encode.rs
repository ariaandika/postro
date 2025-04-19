use crate::postgres::{PgType, Oid};
use crate::value::ValueRef;

/// Value that can be encoded to be bound to sql parameter.
pub trait Encode<'q> {
    fn encode(self) -> Encoded<'q>;
}

/// Postgres encoded value.
#[derive(Debug)]
pub struct Encoded<'q> {
    value: ValueRef<'q>,
    oid: Oid,
}

impl<'q> Encoded<'q> {
    pub(crate) fn new(value: ValueRef<'q>, oid: Oid) -> Self {
        Self { value, oid }
    }

    pub(crate) fn into_value(self) -> ValueRef<'q> {
        self.value
    }

    pub(crate) fn value(&self) -> &ValueRef<'q> {
        &self.value
    }

    pub fn oid(&self) -> Oid {
        self.oid
    }
}

impl Default for Encoded<'_> {
    fn default() -> Self {
        Self {
            value: ().into(),
            oid: <()>::OID,
        }
    }
}

macro_rules! encode {
    (<$lf:tt>$ty:ty) => {
        impl<$lf> Encode<$lf> for &$lf $ty {
            fn encode(self) -> Encoded<$lf> {
                Encoded { value: self.into(), oid: <$ty>::OID, }
            }
        }
    };
    ($ty:ty) => {
        impl Encode<'static> for $ty {
            fn encode(self) -> Encoded<'static> {
                Encoded { value: self.into(), oid: Self::OID, }
            }
        }
    };
}

encode!(bool);
encode!(i32);
encode!(<'a> str);
encode!(<'a> String);

