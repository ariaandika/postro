use std::fmt;

/// Error when parsing config.
pub enum ConfigError {
    /// Error parsing url.
    Parse(&'static str),
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

