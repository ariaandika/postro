//! Type integration with external types
//!
//! Implementation [`Decode`][d], [`Encode`][e], and [`FromRow`][f] for external types.
//!
//! Available for:
//!
//! - [`serde`]'s [`Deserialize`][sd] and [`Serialize`][ss] via [`Json`], requires `json` feature
//! - [`time`][::time]'s [`PrimitiveDateTime`][tp], [`UtcDateTime`][tu], requires `time` feature
//!
//! [d]: crate::Decode
//! [e]: crate::Encode
//! [f]: crate::FromRow
//! [sd]: serde::Deserialize
//! [ss]: serde::Serialize
//! [tp]: ::time::PrimitiveDateTime
//! [tu]: ::time::UtcDateTime

#[cfg(feature = "json")]
mod json;
#[cfg(feature = "json")]
pub use json::Json;

#[cfg(feature = "time")]
mod time;

