
// modules are separated in 3 categories:
// - foundations
// - driver traits
// - toolkit

// NOTE: foundations
pub mod ext;
pub mod io;
pub mod net;
pub mod decode;
pub mod encode;
pub mod types;
pub mod from_row;
pub mod describe;
pub mod executor;
pub mod sync;
pub mod error;

// NOTE: driver traits
pub mod database;
pub mod connection;
pub mod statement;
pub mod arguments;
pub mod type_info;
pub mod row;
pub mod column;
pub mod value;

// NOTE: toolkit
pub mod migrate;

pub use error::{Error, Result};

