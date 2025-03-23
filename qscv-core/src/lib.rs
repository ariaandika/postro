
// modules are separated in 3 categories:
// - foundations
// - driver traits
// - toolkit

// NOTE: foundations
pub mod ext;
pub mod io;
pub mod net;
pub mod error;

// NOTE: driver traits
pub mod types;
pub mod database;
pub mod type_info;

// NOTE: toolkit
pub mod migrate;

pub use error::{Error, Result};

