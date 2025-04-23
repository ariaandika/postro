//! Postgres Driver

mod common;
mod io;
mod net;
mod ext;

// Protocol
pub mod postgres;

// Codec
mod value;
mod encode;
mod column;
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
mod protocol;
pub mod query;

mod error;


pub use encode::{Encode, Encoded};
pub use column::{Column, ColumnInfo, Index};
pub use decode::Decode;
pub use row::{FromRow, Row};

pub use options::PgOptions;
pub use connection::PgConnection;
pub use query::query;

pub use postgres::{ProtocolError, ErrorResponse, NoticeResponse};
pub use error::{Error, Result};

