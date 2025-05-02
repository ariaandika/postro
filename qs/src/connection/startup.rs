use std::borrow::Cow;

use super::Config;

/// A config for postgres startup phase.
///
/// <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-START-UP>
pub struct StartupConfig<'a> {
    user: Cow<'a,str>,
    database: Option<Cow<'a,str>>,
    password: Option<Cow<'a,str>>,
    replication: Option<Cow<'a,str>>,
}

impl<'a> StartupConfig<'a> {
    /// Create new config, the database user name is required.
    pub fn new(user: impl Into<Cow<'a, str>>) -> Self {
        Self { user: user.into(), database: None, password: None, replication: None  }
    }

    /// The database user name to connect as.
    pub fn user(&self) -> &str {
        &self.user
    }

    /// The database to connect to. Defaults to the user name.
    pub fn database(&self) -> Option<&str> {
        self.database.as_ref().map(<_>::as_ref)
    }

    /// The database to connect to. Defaults to the user name.
    pub fn set_database(mut self, database: impl Into<Cow<'a,str>>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Authentication password, the default is empty string.
    pub fn password(&self) -> Option<&str> {
        self.password.as_ref().map(<_>::as_ref)
    }

    /// Authentication password, the default is empty string.
    pub fn set_password(mut self, password: impl Into<Cow<'a,str>>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Used to connect in streaming replication mode, where a small set of replication commands can be issued
    /// instead of SQL statements.
    ///
    /// Value can be true, false, or database, and the default is false.
    ///
    /// See [Section 53.4](https://www.postgresql.org/docs/current/protocol-replication.html) for details.
    pub fn replication(&self) -> Option<&str> {
        self.replication.as_ref().map(<_>::as_ref)
    }

    /// Used to connect in streaming replication mode, where a small set of replication commands can be issued
    /// instead of SQL statements.
    ///
    /// Value can be true, false, or database, and the default is false.
    ///
    /// See [Section 53.4](https://www.postgresql.org/docs/current/protocol-replication.html) for details.
    pub fn set_replication(mut self, replication: impl Into<Cow<'a,str>>) -> Self {
        self.replication = Some(replication.into());
        self
    }
}

pub struct StartupConfigBuilder<'a> {
    config: StartupConfig<'a>,
}

impl<'a> StartupConfigBuilder<'a> {
    /// Create new config builder, the database user name is required.
    pub fn new(user: impl Into<Cow<'a, str>>) -> Self {
        Self { config: StartupConfig::new(user) }
    }

    /// The database to connect to. Defaults to the user name.
    pub fn database(mut self, database: impl Into<Cow<'a,str>>) -> Self {
        self.config.database = Some(database.into());
        self
    }

    /// Authentication password, the default is empty string.
    pub fn password(mut self, password: impl Into<Cow<'a,str>>) -> Self {
        self.config.password = Some(password.into());
        self
    }

    /// Used to connect in streaming replication mode, where a small set of replication commands can be issued
    /// instead of SQL statements.
    ///
    /// Value can be true, false, or database, and the default is false.
    ///
    /// See [Section 53.4](https://www.postgresql.org/docs/current/protocol-replication.html) for details.
    pub fn replication(mut self, replication: impl Into<Cow<'a,str>>) -> Self {
        self.config.replication = Some(replication.into());
        self
    }

    /// Finish builder, returns [`StartupConfig`].
    pub fn build(self) -> StartupConfig<'a> {
        self.config
    }
}

impl<'a> From<&'a Config> for StartupConfig<'a> {
    fn from(me: &'a Config) -> StartupConfig<'a> {
        StartupConfig::new(me.user.as_ref())
            .set_database(me.dbname.as_ref())
            .set_password(me.pass.as_ref())
    }
}

impl<'a> From<StartupConfigBuilder<'a>> for StartupConfig<'a> {
    fn from(value: StartupConfigBuilder<'a>) -> Self {
        value.config
    }
}

