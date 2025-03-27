use std::fmt;

pub type Result<T,E = Error> = std::result::Result<T,E>;

#[derive(Debug)]
pub enum Error {
    Configuration(String),
    Other(Box<dyn std::error::Error + Send + Sync>)
}

impl std::error::Error for Error { }

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Configuration(e) => fmt::Display::fmt(e, f),
            Error::Other(e) => fmt::Display::fmt(e, f)
        }
    }
}


