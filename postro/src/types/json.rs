use bytes::Buf;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    Decode, DecodeError, Encode,
    encode::Encoded,
    postgres::{Oid, PgType},
    row::Column,
};

/// Decode and Encode postgres json value.
///
/// # Panics
///
/// Note that when performing [`Encode`], if [`Serialize`] implementation decide
/// to fail, it will will panics.
#[derive(Debug)]
pub struct Json<T>(pub T);

impl<T> PgType for Json<T> {
    /// jsonb, Binary JSON
    const OID: Oid = 3802;
}

impl<T> Decode for Json<T>
where
    T: DeserializeOwned,
{
    fn decode(column: Column) -> Result<Self, DecodeError> {
        if column.oid() != Self::OID {
            return Err(DecodeError::OidMissmatch);
        }
        let mut value = column.into_value();
        assert_eq!(value.get_u8(), b'\x01', "jsonb version");
        serde_json::from_slice(&value).map_err(Into::into)
    }
}

impl<T: Serialize> Encode<'static> for Json<T> {
    fn encode(self) -> Encoded<'static> {
        Encoded::owned(serde_json::to_vec(&self).unwrap(), Self::OID)
    }
}

impl<T: Serialize> Serialize for Json<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Json<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self(T::deserialize(deserializer)?))
    }
}

