//! Postgres Driver

pub mod common;
mod io;
mod net;
mod ext;

// Protocol
pub mod postgres;

// Codec
mod value;
pub mod column;
pub mod encode;
pub mod decode;

// Component
mod dberror;
mod statement;
pub mod sql;
pub mod row;

// Operation
pub mod executor;
pub mod transport;
pub mod query;

// Connection
pub mod options;
mod connection;

mod error;


pub use encode::Encode;
pub use decode::Decode;
pub use row::FromRow;

pub use options::PgOptions;
pub use connection::PgConnection;
pub use query::{query, execute};

pub use postgres::{ProtocolError, ErrorResponse, NoticeResponse};
pub use error::{Error, Result};

