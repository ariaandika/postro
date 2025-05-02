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
pub mod transport;
pub mod executor;
pub mod query;
pub mod transaction;

// Connection
pub mod connection;
pub mod pool;

mod error;


pub use encode::Encode;
pub use row::FromRow;
pub use sql::SqlExt;

pub use executor::Executor;
pub use connection::{Connection, Config};
pub use pool::{Pool, PoolConfig};
pub use query::{query, execute, begin};

pub use postgres::{ProtocolError, ErrorResponse, NoticeResponse};
pub use error::{Error, Result};

