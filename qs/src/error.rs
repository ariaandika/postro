use std::io;

use crate::{
    common::BoxError,
    postgres::error::{DatabaseError, ProtocolError},
};

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// all possible error from qscv
#[derive(thiserror::Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Configuration(#[source] BoxError),

    #[error("{0}")]
    Protocol(#[from]#[source] ProtocolError),

    #[error("Io error: {0}")]
    Io(#[from]#[source] io::Error),

    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    #[error(transparent)]
    Other(BoxError)
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
macro_rules! err {
    ($variant:ident,$source:ident) => {
        Err(crate::error::Error::$variant($source.into()))
    };
    ($variant:ident,$str:literal,$($tt:tt)*) => {
        Err(crate::error::Error::$variant(err!($str,$($tt)*).into()))
    };
    ($variant:ident,$($tt:tt)*) => {
        Err(crate::error::Error::$variant($($tt)*.into()))
    };
    ($($tt:tt)*) => {
        crate::common::GeneralError::new(format!($($tt)*))
    };
}

pub(crate) use err;

