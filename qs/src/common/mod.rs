//! Supporting utility type.

mod bytestr;
mod url;

pub use bytestr::ByteStr;
pub(crate) use url::{Url, ParseError};

macro_rules! trace {
    ($($tt:tt)*) => {
        #[cfg(feature = "log-verbose")] log::trace!($($tt)*)
    };
}

pub(crate) use trace;

