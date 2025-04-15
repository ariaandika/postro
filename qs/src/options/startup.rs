//! An option for postgres startup phase
//!
//! <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-START-UP>
use std::borrow::Cow;

/// An option for postgres startup phase
///
/// <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-START-UP>
pub struct StartupOptions<'a> {
    user: Cow<'a,str>,
    database: Option<Cow<'a,str>>,
    password: Option<Cow<'a,str>>,
    replication: Option<Cow<'a,str>>,
}

impl<'a> StartupOptions<'a> {
    /// Create new options, the database user name is required
    pub fn new(user: impl Into<Cow<'a, str>>) -> Self {
        Self { user: user.into(), database: None, password: None, replication: None  }
    }

    /// The database user name to connect as. Required; there is no default.
    pub fn get_user(&self) -> &str {
        &self.user
    }

    /// The database to connect to. Defaults to the user name.
    pub fn get_database(&self) -> Option<&str> {
        self.database.as_ref().map(<_>::as_ref)
    }

    /// The database to connect to. Defaults to the user name.
    pub fn database(mut self, database: impl Into<Cow<'a,str>>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Get password
    ///
    /// note that [`None`] is assumed empty string password
    pub fn get_password(&self) -> Option<&str> {
        self.password.as_ref().map(<_>::as_ref)
    }

    /// Set password
    ///
    /// note that [`None`] is assumed empty string password
    pub fn password(mut self, password: impl Into<Cow<'a,str>>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Used to connect in streaming replication mode, where a small set of replication commands can be issued
    /// instead of SQL statements.
    ///
    /// Value can be true, false, or database, and the default is false.
    ///
    /// See [Section 53.4](https://www.postgresql.org/docs/current/protocol-replication.html) for details.
    pub fn get_replication(&self) -> Option<&str> {
        self.replication.as_ref().map(<_>::as_ref)
    }

    /// Set replication
    ///
    /// Used to connect in streaming replication mode, where a small set of replication commands can be issued
    /// instead of SQL statements.
    ///
    /// Value can be true, false, or database, and the default is false.
    ///
    /// See [Section 53.4](https://www.postgresql.org/docs/current/protocol-replication.html) for details.
    pub fn replication(mut self, replication: impl Into<Cow<'a,str>>) -> Self {
        self.replication = Some(replication.into());
        self
    }
}

