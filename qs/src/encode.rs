use bytes::Buf;

use crate::{
    ext::BindParams,
    postgres::{Oid, PgType},
    value::ValueRef,
};

/// Value that can be encoded to be bound to sql parameter.
pub trait Encode<'q> {
    fn encode(self) -> Encoded<'q>;
}

/// Postgres encoded value.
#[derive(Debug)]
pub struct Encoded<'q> {
    value: ValueRef<'q>,
    is_null: bool,
    oid: Oid,
}

impl<'q> Encoded<'q> {
    pub(crate) fn new(value: ValueRef<'q>, oid: Oid) -> Self {
        Self { value, oid, is_null: false, }
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

impl Buf for Encoded<'_> {
    fn remaining(&self) -> usize {
        self.value.remaining()
    }

    fn chunk(&self) -> &[u8] {
        self.value.chunk()
    }

    fn advance(&mut self, cnt: usize) {
        self.value.advance(cnt);
    }
}

impl BindParams for Encoded<'_> {
    fn size(&self) -> i32 {
        match self.is_null {
            true => -1,
            false => self.remaining().try_into().unwrap(),
        }
    }
}

macro_rules! encode {
    (<$lf:tt>$ty:ty) => {
        impl<$lf> Encode<$lf> for &$lf $ty {
            fn encode(self) -> Encoded<$lf> {
                Encoded { value: self.into(), oid: <$ty>::OID, is_null: false, }
            }
        }
    };
    ($ty:ty) => {
        impl Encode<'static> for $ty {
            fn encode(self) -> Encoded<'static> {
                Encoded { value: self.into(), oid: Self::OID, is_null: false, }
            }
        }
    };
}

encode!(bool);
encode!(i32);
encode!(<'a> str);
encode!(<'a> String);

