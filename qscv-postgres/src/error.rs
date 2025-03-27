use std::io;

pub type Result<T,E = Error> = std::result::Result<T,E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("configuration error: {0}")]
    Configuration(String),

    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync>)
}

