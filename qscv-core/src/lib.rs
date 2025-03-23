
// modules are separated in 3 categories:
// - foundations
// - driver traits
// - toolkit

// NOTE: foundations
pub mod io;
pub mod net;
pub mod error;

// NOTE: driver traits
pub mod database;
pub mod type_info;
pub mod types;

// NOTE: toolkit
pub mod migrate;

pub use error::{Error, Result};

