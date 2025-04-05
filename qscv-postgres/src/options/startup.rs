use std::borrow::Cow;

// An option for postgres startup phase
//
// <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-START-UP>
pub struct StartupOptions<'a> {
    user: Cow<'a,str>,
    database: Option<Cow<'a,str>>,
    password: Option<Cow<'a,str>>,
    replication: Option<Cow<'a,str>>,
}

impl<'a> StartupOptions<'a> {
    /// create a builder
    pub fn builder(user: impl Into<Cow<'a, str>>) -> StartupOptionsBuilder<'a> {
        StartupOptionsBuilder::new(user)
    }

    /// get user
    pub fn user(&self) -> &str {
        &self.user
    }

    /// get database
    pub fn database(&self) -> Option<&Cow<'a, str>> {
        self.database.as_ref()
    }

    /// get password
    pub fn password(&self) -> Option<&Cow<'a, str>> {
        self.password.as_ref()
    }

    /// get replication
    pub fn replication(&self) -> Option<&Cow<'a, str>> {
        self.replication.as_ref()
    }
}

/// Builder got [`StartupOptions`]
pub struct StartupOptionsBuilder<'a> {
    user: Cow<'a,str>,
    database: Option<Cow<'a,str>>,
    password: Option<Cow<'a,str>>,
    replication: Option<Cow<'a,str>>,
}

impl<'a> StartupOptionsBuilder<'a> {
    /// create new builder
    pub fn new(user: impl Into<Cow<'a, str>>) -> Self {
        Self {
            user: user.into(),
            database: None,
            password: None,
            replication: None,
        }
    }

    /// set database
    pub fn database(mut self, database: impl Into<Cow<'a,str>>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// set password
    pub fn password(mut self, password: impl Into<Cow<'a,str>>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// set replication
    pub fn replication(mut self, replication: impl Into<Cow<'a,str>>) -> Self {
        self.replication = Some(replication.into());
        self
    }

    /// build the final options
    pub fn build(self) -> StartupOptions<'a> {
        StartupOptions {
            user: self.user,
            database: self.database,
            password: self.password,
            replication: self.replication,
        }
    }
}

impl<'a> From<StartupOptionsBuilder<'a>> for StartupOptions<'a> {
    fn from(opt: StartupOptionsBuilder<'a>,) -> StartupOptions<'a> {
        opt.build()
    }
}

