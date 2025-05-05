//! Supporting utility type.
mod bytestr;
pub use bytestr::ByteStr;

/// Trace when `verbose` feature enabled.
macro_rules! verbose {
    ($($tt:tt)*) => {
        #[cfg(feature = "verbose")]
        tracing::trace!($($tt)*)
    };
}

/// Create and enter `Span` when `verbose` feature enabled.
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

