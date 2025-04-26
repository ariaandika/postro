//! Sql string operation.

/// Type that represent sql string.
pub trait Sql {
    fn sql(&self) -> &str;

    fn persistent(&self) -> bool;
}

impl<'me> Sql for &'me str {
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
    fn once(self) -> SqlOnce<'a>;

    fn persistent(self) -> &'a str;
}

impl<'a> SqlExt<'a> for &'a str {
    fn once(self) -> SqlOnce<'a> {
        SqlOnce(self)
    }

    fn persistent(self) -> &'a str {
        self
    }
}

impl<'a> SqlExt<'a> for SqlOnce<'a> {
    fn once(self) -> SqlOnce<'a> {
        self
    }

    fn persistent(self) -> &'a str {
        self.0
    }
}

