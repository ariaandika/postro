
// modules are separated in 3 categories:
// - foundations
// - core
// - driver traits
// - toolkit

// NOTE: foundations
pub mod ext;
pub mod io;
pub mod net;
pub mod rt;
pub mod sync;
pub mod describe;
pub mod error;

// NOTE: core traits
pub mod decode;
pub mod encode;
pub mod types;
pub mod from_row;
pub mod executor;

// NOTE: query
pub mod query;
pub mod query_as;
pub mod query_scalar;

// NOTE: driver traits
pub mod database;
pub mod acquire;
pub mod connection;
pub mod transaction;
pub mod statement;
pub mod arguments;
pub mod type_info;
pub mod row;
pub mod column;
pub mod value;

// NOTE: toolkit
#[cfg(feature = "migration")]
pub mod migration;
pub mod pool;

pub use error::{Error, Result};

