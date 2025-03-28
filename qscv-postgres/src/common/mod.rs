pub mod bytestr;
pub mod url;

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

