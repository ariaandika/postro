
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
pub mod decode;
pub mod encode;
pub mod types;
pub mod type_info;
pub mod database;
pub mod connection;
pub mod arguments;
pub mod row;
pub mod column;
pub mod value;

// NOTE: toolkit
pub mod migrate;

pub use error::{Error, Result};

