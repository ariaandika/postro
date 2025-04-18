
/// Postgres data transmission format.
///
/// For specific information, see its variant documentation.
///
/// In this library, all format uses [`Binary`][b].
///
/// <https://www.postgresql.org/docs/current/protocol-overview.html#PROTOCOL-FORMAT-CODES>
///
/// [t]: PgFormat::Text
/// [b]: PgFormat::Binary
#[derive(Debug)]
pub enum PgFormat {
    /// Text has format code zero.
    ///
    /// In the [`Text`][t] transmitted representation, there is no trailing null character;
    /// the frontend must add one to received values if it wants to process them as C strings.
    /// (The [`Text`][t] format does not allow embedded nulls, by the way.)
    ///
    /// [t]: PgFormat::Text
    Text,
    /// Binary has format code one.
    ///
    /// [`Binary`][b] representations for integers use network byte order (most significant byte first).
    /// For other data types consult the documentation or source code to learn about the binary representation.
    /// Keep in mind that binary representations for complex data types might change across server versions.
    ///
    /// [b]: PgFormat::Binary
    Binary,
}

impl PgFormat {
    /// Return format code for current format.
    pub fn format_code(&self) -> u16 {
        match self {
            PgFormat::Text => 0,
            PgFormat::Binary => 1,
        }
    }
}


