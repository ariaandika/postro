use crate::{
    common::{bytestr::ByteStr, url::Url},
    error::err,
    Result,
};

/// postgres connection options
#[derive(Debug)]
#[allow(unused)]
pub struct PgOptions {
    pub(crate) user: ByteStr,
    pub(crate) pass: ByteStr,
    pub(crate) socket: Option<ByteStr>,
    pub(crate) host: ByteStr,
    pub(crate) port: u16,
    pub(crate) dbname: ByteStr,
}

impl PgOptions {
    pub fn new() {
        todo!("postgres env var convention")
    }

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

