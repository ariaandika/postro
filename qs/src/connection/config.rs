//! Postgres configuration.
use crate::common::ByteStr;
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

        let mut read = url.as_str();
        macro_rules! eat {
            (@ $delim:literal,$id:tt,$len:literal) => {{
                let Some(idx) = read.find($delim) else {
                    return Err(ConfigError::Parse(concat!(stringify!($id), " missing")))
                };
                let capture = &read[..idx];
                read = &read[idx + $len..];
                url.slice_ref(capture)
            }};
            ($delim:literal,$id:tt) => {
                eat!(@ $delim,$id,1)
            };
            ($delim:literal,$id:tt,$len:literal) => {
                eat!(@ $delim,$id,$len)
            };
        }

        let _scheme = eat!("://", user, 3);
        let user = eat!(':', password);
        let pass = eat!('@', host);
        let host = eat!(':', port);
        let port = eat!('/', dbname);
        let dbname = url.slice_ref(read);

        let Ok(port) = port.parse() else {
            return Err(ConfigError::Parse("invalid port"))
        };

        Ok(Self { user, pass, host, port, dbname, socket: None })
    }
}

impl std::str::FromStr for PgConfig {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

