
/// Postgres object identifier.
///
/// The oid type is implemented as an unsigned four-byte integer.
///
/// <https://www.postgresql.org/docs/current/datatype-oid.html>
pub type Oid = u32;

/// A type that have corresponding postgred oid.
pub trait PgType {
    const OID: Oid;
}

// json, 114, "JSON stored as text"
// jsonb, 3802, "Binary JSON"
// date, 1082, "date"
// time, 1083, "time of day"
// timestamp, 1114, "date and time"
// timestamptz, 1184, "date and time with timezone"

macro_rules! oid {
    ($ty:ty, $oid:literal $(, $doc:literal)? ) => {
        impl PgType for $ty {
            $(#[doc = $doc])?
            const OID: Oid = $oid;
        }
    };
}

// oid!((), 0); // 0 means type unspecified
oid!(bool, 16);
oid!(char, 18);
oid!(i64, 20, "`int8` ~18 digit integer, 8-byte storage");
oid!(i16, 21, "`int2` -32 thousand to 32 thousand, 2-byte storage");
oid!(i32, 23, "`int4` -2 billion to 2 billion integer, 4-byte storage");
oid!(str, 25, "`text` variable-length string, no limit specified");
oid!(String, 25, "`text` variable-length string, no limit specified");
oid!(f32, 700, "`float4` single-precision floating point number, 4-byte storage");
oid!(f64, 701, "`float8` double-precision floating point number, 8-byte storage");


#[cfg(feature = "time")]
impl PgType for time::PrimitiveDateTime {
    /// date and time
    const OID: Oid = 1114;
}

