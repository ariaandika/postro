pub type Result<T,E = Error> = std::result::Result<T,E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("configuration error: {0}")]
    Configuration(String),

    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync>)
}

