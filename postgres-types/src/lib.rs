use std::fmt;

use type_gen::Inner;

mod pg_lsn;
mod type_gen;

pub use postgres_protocol::Oid;

/// A trait for types that can be created from a Postgres value.
///
/// # Types
///
/// The following implementations are provided by this crate, along with the
/// corresponding Postgres types:
///
/// | Rust type                         | Postgres type(s)                              |
/// |-----------------------------------|-----------------------------------------------|
/// | `bool`                            | BOOL                                          |
/// | `i8`                              | "char"                                        |
/// | `i16`                             | SMALLINT, SMALLSERIAL                         |
/// | `i32`                             | INT, SERIAL                                   |
/// | `u32`                             | OID                                           |
/// | `i64`                             | BIGINT, BIGSERIAL                             |
/// | `f32`                             | REAL                                          |
/// | `f64`                             | DOUBLE PRECISION                              |
/// | `&str`/`String`                   | VARCHAR, CHAR(n), TEXT, CITEXT, NAME, UNKNOWN |
/// |                                   | LTREE, LQUERY, LTXTQUERY                      |
/// | `&[u8]`/`Vec<u8>`                 | BYTEA                                         |
/// | `HashMap<String, Option<String>>` | HSTORE                                        |
/// | `SystemTime`                      | TIMESTAMP, TIMESTAMP WITH TIME ZONE           |
/// | `IpAddr`                          | INET                                          |
///
/// In addition, some implementations are provided for types in third party
/// crates. These are disabled by default; to opt into one of these
/// implementations, activate the Cargo feature corresponding to the crate's
/// name prefixed by `with-`. For example, the `with-serde_json-1` feature enables
/// the implementation for the `serde_json::Value` type.
///
/// | Rust type                       | Postgres type(s)                    |
/// |---------------------------------|-------------------------------------|
/// | `chrono::NaiveDateTime`         | TIMESTAMP                           |
/// | `chrono::DateTime<Utc>`         | TIMESTAMP WITH TIME ZONE            |
/// | `chrono::DateTime<Local>`       | TIMESTAMP WITH TIME ZONE            |
/// | `chrono::DateTime<FixedOffset>` | TIMESTAMP WITH TIME ZONE            |
/// | `chrono::NaiveDate`             | DATE                                |
/// | `chrono::NaiveTime`             | TIME                                |
/// | `cidr::IpCidr`                  | CIDR                                |
/// | `cidr::IpInet`                  | INET                                |
/// | `time::PrimitiveDateTime`       | TIMESTAMP                           |
/// | `time::OffsetDateTime`          | TIMESTAMP WITH TIME ZONE            |
/// | `time::Date`                    | DATE                                |
/// | `time::Time`                    | TIME                                |
/// | `jiff::civil::Date`             | DATE                                |
/// | `jiff::civil::DateTime`         | TIMESTAMP                           |
/// | `jiff::civil::Time`             | TIME                                |
/// | `jiff::Timestamp`               | TIMESTAMP WITH TIME ZONE            |
/// | `eui48::MacAddress`             | MACADDR                             |
/// | `geo_types::Point<f64>`         | POINT                               |
/// | `geo_types::Rect<f64>`          | BOX                                 |
/// | `geo_types::LineString<f64>`    | PATH                                |
/// | `serde_json::Value`             | JSON, JSONB                         |
/// | `uuid::Uuid`                    | UUID                                |
/// | `bit_vec::BitVec`               | BIT, VARBIT                         |
/// | `eui48::MacAddress`             | MACADDR                             |
/// | `cidr::InetCidr`                | CIDR                                |
/// | `cidr::InetAddr`                | INET                                |
/// | `smol_str::SmolStr`             | VARCHAR, CHAR(n), TEXT, CITEXT,     |
/// |                                 | NAME, UNKNOWN, LTREE, LQUERY,       |
/// |                                 | LTXTQUERY                           |
///
/// # Nullability
///
/// In addition to the types listed above, `FromSql` is implemented for
/// `Option<T>` where `T` implements `FromSql`. An `Option<T>` represents a
/// nullable Postgres value.
///
/// # Arrays
///
/// `FromSql` is implemented for `Vec<T>`, `Box<[T]>` and `[T; N]` where `T`
/// implements `FromSql`, and corresponds to one-dimensional Postgres arrays.
///
/// **Note:** the impl for arrays only exist when the Cargo feature `array-impls`
/// is enabled.
pub trait FromSql<'a>: Sized {
    /// Creates a new value of this type from a buffer of data of the specified
    /// Postgres `Type` in its binary format.
    ///
    /// The caller of this method is responsible for ensuring that this type
    /// is compatible with the Postgres `Type`.
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn std::error::Error + Sync + Send>>;

    /// Creates a new value of this type from a `NULL` SQL value.
    ///
    /// The caller of this method is responsible for ensuring that this type
    /// is compatible with the Postgres `Type`.
    ///
    /// The default implementation returns `Err(Box::new(WasNull))`.
    #[allow(unused_variables)]
    fn from_sql_null(ty: &Type) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        Err(Box::new(WasNull))
    }

    /// A convenience function that delegates to `from_sql` and `from_sql_null` depending on the
    /// value of `raw`.
    fn from_sql_nullable(
        ty: &Type,
        raw: Option<&'a [u8]>,
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        match raw {
            Some(raw) => Self::from_sql(ty, raw),
            None => Self::from_sql_null(ty),
        }
    }

    /// Determines if a value of this type can be created from the specified
    /// Postgres `Type`.
    fn accepts(ty: &Type) -> bool;
}

/// A Postgres type.
#[derive(PartialEq, Eq, Clone, Hash)]
pub struct Type(Inner);

/// An error indicating that a `NULL` Postgres value was passed to a `FromSql`
/// implementation that does not support `NULL` values.
#[derive(Debug, Clone, Copy)]
pub struct WasNull;

impl std::error::Error for WasNull {}

impl fmt::Display for WasNull {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("a Postgres value was `NULL`")
    }
}



