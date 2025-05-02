//! Query parameter encoding.
use bytes::{Buf, Bytes};

use crate::{
    ext::BindParams,
    postgres::{Oid, PgType},
    value::ValueRef,
};

/// Value that can be encoded to be bound to sql parameter.
pub trait Encode<'q> {
    /// Encode the value.
    fn encode(self) -> Encoded<'q>;
}

/// Postgres encoded value.
pub struct Encoded<'q> {
    value: ValueRef<'q>,
    is_null: bool,
    oid: Oid,
}

impl<'q> Encoded<'q> {
    /// Create [`Encoded`] from borrowed slice.
    pub fn from_slice(slice: &'q [u8], oid: Oid) -> Encoded<'q> {
        Encoded {
            value: ValueRef::Slice(slice),
            is_null: false,
            oid,
        }
    }

    /// Create heap allocated [`Encoded`] by copying given slice.
    pub fn copy_from_slice(slice: &[u8], oid: Oid) -> Encoded<'static> {
        Encoded {
            value: ValueRef::Bytes(Bytes::copy_from_slice(slice)),
            is_null: false,
            oid,
        }
    }

    /// Create [`Encoded`] `NULL`.
    pub fn null() -> Encoded<'static> {
        Encoded {
            value: ValueRef::Slice(&[]),
            is_null: true,
            oid: 0,
        }
    }

    /// Returns this type `oid`, or `0` for `NULL`.
    pub fn oid(&self) -> Oid {
        match self.is_null {
            true => 0,
            false => self.oid,
        }
    }

    pub(crate) fn value(&self) -> &ValueRef<'q> {
        &self.value
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
    (<$lf:tt,$ty:ty>$pat:tt => $body:expr) => {
        impl<$lf> Encode<$lf> for &$lf $ty {
            fn encode($pat) -> Encoded<$lf> {
                Encoded {
                    value: $body,
                    oid: <$ty>::OID,
                    is_null: false,
                }
            }
        }
    };
    (<$ty:ty>$pat:tt => $body:expr) => {
        impl Encode<'static> for $ty {
            fn encode($pat) -> Encoded<'static> {
                Encoded {
                    value: $body,
                    oid: <$ty>::OID,
                    is_null: false,
                }
            }
        }
    };
}

encode!(<bool>self => ValueRef::inline(&(self as u8).to_be_bytes()));
encode!(<i32>self => ValueRef::inline(&self.to_be_bytes()));
encode!(<'a,str>self => ValueRef::Slice(self.as_bytes()));
encode!(<'a,String>self => ValueRef::Slice(self.as_bytes()));

impl std::fmt::Debug for Encoded<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("Encoded")
            .field(if self.is_null { &"NULL" } else { &self.value })
            .field(&self.oid)
            .finish()
    }
}

