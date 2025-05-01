use std::fmt;

use crate::common::ParseError;

/// Error when parsing config.
pub enum ConfigError {
    /// Error parsing url.
    Parse(String),
}

impl std::error::Error for ConfigError { }

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Parse(e) => write!(f, "Config error: {e}"),
        }
    }
}

impl fmt::Debug for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{self}\"")
    }
}

macro_rules! from {
    (<$ty:ty>$pat:pat => $body:expr) => {
        impl From<$ty> for ConfigError {
            fn from($pat: $ty) -> Self {
                $body
            }
        }
    };
}

from!(<ParseError>e => Self::Parse(e.to_string()));

