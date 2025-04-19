
pub type Oid = u32;

pub trait PgType {
    const OID: Oid;
}

impl<T> PgType for &T where T: PgType {
    const OID: Oid = T::OID;
}

// Self::Int4 => 23,
// Self::Date => 1082,

impl PgType for () {
    const OID: Oid = 0;
}

impl PgType for bool {
    const OID: Oid = 16;
}

impl PgType for i32 {
    const OID: Oid = 20;
}

impl PgType for str {
    const OID: Oid = 25;
}

impl PgType for String {
    const OID: Oid = 25;
}

