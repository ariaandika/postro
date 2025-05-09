//! Supporting utility type.
mod bytestr;
pub use bytestr::ByteStr;

/// Create unit type `Error`.
///
/// # Example
///
/// ```ignore
/// unit_error! {
///     /// Resource not found.
///     pub struct NotFound("not found");
/// }
/// ```
macro_rules! unit_error {
    ($(#[$meta:meta])* $vis:vis struct $name:ident($msg:literal);) => {
        $(#[$meta])*
        $vis struct $name;

        impl std::error::Error for $name { }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str($msg)
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "\"{self}\"")
            }
        }
    };
}

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

pub(crate) use unit_error;
pub(crate) use verbose;
pub(crate) use span;

