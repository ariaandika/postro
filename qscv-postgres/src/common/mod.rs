mod bytestr;
mod general;
mod url;

pub use bytestr::ByteStr;
pub use general::GeneralError;
pub use url::Url;

pub(crate) use general::general;

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Copied from `Bytes` `Debug` implementation
pub struct BytesRef<'a>(pub &'a [u8]);
impl std::fmt::Debug for BytesRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "b\"")?;
        for &b in self.0 {
            if b == b'\n' {
                write!(f, "\\n")?;
            } else if b == b'\r' {
                write!(f, "\\r")?;
            } else if b == b'\t' {
                write!(f, "\\t")?;
            } else if b == b'\\' || b == b'"' {
                write!(f, "\\{}", b as char)?;
            } else if b == b'\0' {
                write!(f, "\\0")?;
            } else if (0x20..0x7f).contains(&b) {
                write!(f, "{}", b as char)?;
            } else {
                write!(f, "\\x{:02x}", b)?;
            }
        }
        write!(f, "\"")?;
        Ok(())
    }
}

/// `Display` implementation for lossy str
pub struct LossyStr<'a>(pub &'a [u8]);

impl std::fmt::Display for LossyStr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for &b in self.0 {
            if b.is_ascii_graphic() {
                write!(f, "{}", b as char)?;
            } else if b.is_ascii_whitespace() {
                write!(f, "{}", b as char)?;
            } else {
                write!(f, "\\x{b:x}")?;
            }
        }
        Ok(())
    }
}

