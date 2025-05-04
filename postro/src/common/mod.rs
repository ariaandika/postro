//! Supporting utility type.
mod bytestr;
pub use bytestr::ByteStr;

macro_rules! verbose {
    ($($tt:tt)*) => {
        #[cfg(feature = "verbose")]
        tracing::trace!($($tt)*)
    };
}

macro_rules! span {
    ($($tt:tt)*) => {
        #[cfg(feature = "verbose")]
        let s = tracing::trace_span!($($tt)*);
        #[cfg(feature = "verbose")]
        let _s = s.enter();
    };
}

pub(crate) use verbose;
pub(crate) use span;

