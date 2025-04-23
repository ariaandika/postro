mod startup;

use crate::{
    common::{ByteStr, Url},
    error::err,
    Result,
};

pub use startup::StartupOptions;

/// postgres connection options
#[derive(Debug)]
pub struct PgOptions {
    pub(crate) user: ByteStr,
    pub(crate) pass: ByteStr,
    #[allow(unused)] // socket used later
    pub(crate) socket: Option<ByteStr>,
    pub(crate) host: ByteStr,
    pub(crate) port: u16,
    pub(crate) dbname: ByteStr,
}

impl<'a> From<&'a PgOptions> for startup::StartupOptions<'a> {
    fn from(me: &'a PgOptions) -> startup::StartupOptions<'a> {
        startup::StartupOptions::new(me.user.as_ref())
            .database(me.dbname.as_ref())
            .password(me.pass.as_ref())
    }
}

impl PgOptions {
    // TODO: postgres env var convention
    // pub fn new() { }

    pub fn parse(url: &str) -> Result<PgOptions> {
        Self::parse_inner(ByteStr::copy_from_str(url))
    }

    pub fn parse_static(url: &'static str) -> Result<PgOptions> {
        Self::parse_inner(ByteStr::from_static(url))
    }

    fn parse_inner(url: ByteStr) -> Result<Self> {
        // TODO: socket path input

        let url = match Url::parse(url) {
            Ok(ok) => ok,
            Err(err) => return err!(Configuration,err),
        };

        if !matches!(url.scheme.as_ref(), "postgres" | "postgresql") {
            return err!(Configuration, "expected schema to be `postgres`");
        }

        let Url { user, pass, host, port, dbname, .. } = url;
        Ok(Self { user, pass, host, port, dbname, socket: None })
    }
}

impl std::str::FromStr for PgOptions {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

