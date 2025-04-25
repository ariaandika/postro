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
mod row;

// Connection
pub mod options;
mod transport;
mod connection;

// Operation
pub mod query;

mod error;


pub use encode::Encode;
pub use decode::Decode;
pub use row::{FromRow, Row};

pub use options::PgOptions;
pub use connection::PgConnection;
pub use query::query;

pub use postgres::{ProtocolError, ErrorResponse, NoticeResponse};
pub use error::{Error, Result};

