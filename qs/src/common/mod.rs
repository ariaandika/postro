mod bytestr;
mod url;

pub use bytestr::ByteStr;
pub use url::Url;

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// `fmt` implementation for lossy str
pub struct LossyStr<'a>(pub &'a [u8]);

impl std::fmt::Display for LossyStr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for &b in self.0 {
            if b.is_ascii_graphic() || b.is_ascii_whitespace() {
                write!(f, "{}", b as char)?;
            } else {
                write!(f, "\\x{b:x}")?;
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for LossyStr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "b\"{self}\"")
    }
}

