//! Sql string operation.

/// Type that represent sql string.
pub trait Sql {
    /// Returns sql string.
    fn sql(&self) -> &str;

    /// Return `true` if current statement should be cached.
    fn persistent(&self) -> bool;
}

impl Sql for &str {
    fn sql(&self) -> &str {
        self
    }

    fn persistent(&self) -> bool {
        true
    }
}

/// Non persistent query string.
#[derive(Debug)]
pub struct SqlOnce<'sql>(&'sql str);

impl Sql for SqlOnce<'_> {
    fn sql(&self) -> &str {
        self.0
    }

    fn persistent(&self) -> bool {
        false
    }
}

/// Extension trait for easier query persistence config.
pub trait SqlExt<'a> {
    /// Disable statement caching.
    fn once(self) -> SqlOnce<'a>;
}

impl<'a> SqlExt<'a> for &'a str {
    fn once(self) -> SqlOnce<'a> {
        SqlOnce(self)
    }
}

impl<'a> SqlExt<'a> for SqlOnce<'a> {
    fn once(self) -> SqlOnce<'a> {
        self
    }
}

