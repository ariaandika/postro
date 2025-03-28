// common utility which completely isolated
mod common;

// Driver
pub mod protocol;

// Runtime
mod net;

// Error
mod error;

// Postgres
pub mod postgres;

pub use self::error::{Result, Error};
pub use self::postgres::prelude::*;
