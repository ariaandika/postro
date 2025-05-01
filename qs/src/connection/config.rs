//! Postgres configuration.
use crate::common::{ByteStr, Url};
use super::{ConfigError, startup};

/// Postgres connection config.
#[derive(Clone, Debug)]
pub struct PgConfig {
    pub(crate) user: ByteStr,
    pub(crate) pass: ByteStr,
    #[allow(unused)] // socket used later
    pub(crate) socket: Option<ByteStr>,
    pub(crate) host: ByteStr,
    pub(crate) port: u16,
    pub(crate) dbname: ByteStr,
}

impl<'a> From<&'a PgConfig> for startup::StartupConfig<'a> {
    fn from(me: &'a PgConfig) -> startup::StartupConfig<'a> {
        startup::StartupConfig::new(me.user.as_ref())
            .database(me.dbname.as_ref())
            .password(me.pass.as_ref())
    }
}

impl PgConfig {
    // TODO: postgres env var convention
    // pub fn new() { }

    pub fn parse(url: &str) -> Result<PgConfig, ConfigError> {
        Self::parse_inner(ByteStr::copy_from_str(url))
    }

    pub fn parse_static(url: &'static str) -> Result<PgConfig, ConfigError> {
        Self::parse_inner(ByteStr::from_static(url))
    }

    fn parse_inner(url: ByteStr) -> Result<Self, ConfigError> {
        // TODO: socket path input

        let url = Url::parse(url)?;

        // if !matches!(url.scheme.as_ref(), "postgres" | "postgresql") {
        //     return err!(Configuration, "expected schema to be `postgres`");
        // }

        let Url { user, pass, host, port, dbname, .. } = url;
        Ok(Self { user, pass, host, port, dbname, socket: None })
    }
}

impl std::str::FromStr for PgConfig {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

