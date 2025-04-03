macro_rules! general {
    (%$str:literal) => {
        crate::common::GeneralError::new($str)
    };
    ($($tt:tt)*) => {
        crate::common::GeneralError::new(format!($($tt)*))
    };
}

use std::borrow::Cow;

pub(crate) use general;

/// an error which only contain string message
///
/// `GeneralError` should be an unrecoverable error, which an error
/// that are meant to be displayed instead of handled in application
pub struct GeneralError(Cow<'static,str>);

impl GeneralError {
    pub fn new(message: impl Into<Cow<'static,str>>) -> GeneralError {
        Self(message.into())
    }
}

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


