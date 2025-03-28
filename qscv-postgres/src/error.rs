use std::io;

use crate::protocol::ProtocolError;

pub type Result<T,E = Error> = std::result::Result<T,E>;

/// all possible error from qscv
#[derive(thiserror::Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Configuration(#[source] Box<dyn std::error::Error>),

    #[error("{0}")]
    Protocol(#[from]#[source] ProtocolError),

    #[error("Io error: {0}")]
    Io(#[from]#[source] io::Error),

    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync>)
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)?;
        // TODO: how to differentiate
        // if let Some(err) = std::error::Error::source(&self) {
        //     write!(f, "\n\nCaused By:\n    {err}")?;
        // }
        Ok(())
    }
}

/// general error return
#[macro_export]
macro_rules! err {
    ($variant:ident,$source:ident) => {
        Err(crate::error::Error::$variant($source.into()))
    };
    ($variant:ident,$($tt:tt)*) => {
        Err(crate::error::Error::$variant(err!($($tt)*).into()))
    };
    ($($tt:tt)*) => {
        crate::error::GeneralError(format!($($tt)*))
    };
}

/// an error which only contain string message
pub struct GeneralError(pub String);

impl std::error::Error for GeneralError { }

impl std::fmt::Display for GeneralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::Debug for GeneralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}


