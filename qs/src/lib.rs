//! Postgres Driver

pub mod common;
mod io;
mod net;
mod ext;

// Protocol
pub mod postgres;

// Encoding
mod value;
pub mod encode;

// Component
mod statement;
pub mod sql;
pub mod row;

// Operation
pub mod executor;
pub mod transport;
pub mod query;
pub mod transaction;

// Connection
pub mod options;
mod connection;

mod error;


pub use encode::Encode;
pub use row::FromRow;

pub use options::PgOptions;
pub use connection::PgConnection;
pub use query::{query, execute};

pub use postgres::{ProtocolError, ErrorResponse, NoticeResponse};
pub use error::{Error, Result};

