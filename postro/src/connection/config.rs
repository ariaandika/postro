//! Postgres configuration.
use std::{borrow::Cow, env::var, fmt};

use crate::{common::ByteStr, phase::StartupConfig};

/// Postgres connection config.
#[derive(Clone, Debug)]
pub struct Config {
    pub(crate) user: ByteStr,
    pub(crate) pass: ByteStr,
    #[allow(unused)] // socket used later
    pub(crate) socket: Option<ByteStr>,
    pub(crate) host: ByteStr,
    pub(crate) port: u16,
    pub(crate) dbname: ByteStr,
}

impl Config {
    /// Retrieve configuration from environment variable.
    ///
    /// It reads:
    /// - `PGUSER`
    /// - `PGPASS`
    /// - `PGHOST`
    /// - `PGDATABASE`
    /// - `PGPORT`
    ///
    /// Additionally, it also read `DATABASE_URL` to provide missing value from
    /// previous variables before fallback to default value.
    pub fn from_env() -> Config {
        let url = var("DATABASE_URL").ok().and_then(|e|Config::parse_inner(e.into()).ok());

        macro_rules! env {
            ($name:literal,$or:ident,$def:expr) => {
                match (var($name),url.as_ref()) {
                    (Ok(ok),_) => ok.into(),
                    (Err(_),Some(e)) => e.$or.clone(),
                    (Err(_),None) => $def.into(),
                }
            };
        }

        let user = env!("PGUSER",user,"postgres");
        let pass = env!("PGPASS",pass,"");
        let host = env!("PGHOST",host,"localhost");
        let dbname = env!("PGDATABASE",dbname,user.clone());
        let socket = url.as_ref().and_then(|e|e.socket.clone());

        let port = match (var("PGPORT"),url.as_ref()) {
            (Ok(ok),_) => ok.parse().unwrap_or(5432),
            (Err(_),Some(e)) => e.port,
            (Err(_),None) => 5432,
        };

        Self { user, pass, socket, host, port, dbname }
    }

    /// Parse config from url.
    pub fn parse(url: &str) -> Result<Config, ParseError> {
        Self::parse_inner(ByteStr::copy_from_str(url))
    }

    /// Parse config from static strign url.
    ///
    /// This is for micro optimization, see [`Bytes::from_static`][1].
    ///
    /// [1]: bytes::Bytes::from_static
    pub fn parse_static(url: &'static str) -> Result<Config, ParseError> {
        Self::parse_inner(ByteStr::from_static(url))
    }

    fn parse_inner(url: ByteStr) -> Result<Self, ParseError> {
        let mut read = url.as_str();

        macro_rules! eat {
            (@ $delim:literal,$id:tt,$len:literal) => {{
                let Some(idx) = read.find($delim) else {
                    return Err(ParseError { reason: concat!(stringify!($id), " missing").into() })
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
            return Err(ParseError { reason: "invalid port".into() })
        };

        Ok(Self { user, pass, host, port, dbname, socket: None })
    }
}

impl<'a> From<&'a Config> for StartupConfig<'a> {
    fn from(me: &'a Config) -> StartupConfig<'a> {
        StartupConfig {
            user: me.user.as_str().into(),
            database: Some(me.user.as_str().into()),
            password: Some(me.pass.as_str().into()),
            replication: None,
        }
    }
}

impl std::str::FromStr for Config {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Error when parsing url.
pub struct ParseError {
    pub(crate) reason: Cow<'static,str>,
}

impl std::error::Error for ParseError { }

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            return f.write_str(&self.reason)
        }
        write!(f, "failed to parse url: {}", self.reason)
    }
}

impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{self}\"")
    }
}

