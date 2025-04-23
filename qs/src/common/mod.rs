mod bytestr;
mod url;

pub use bytestr::ByteStr;
pub use url::Url;

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

