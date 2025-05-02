//! Supporting utility type.

mod bytestr;

pub use bytestr::ByteStr;

macro_rules! trace {
    ($($tt:tt)*) => {
        #[cfg(feature = "log-verbose")] log::trace!($($tt)*)
    };
}

pub(crate) use trace;

