//! Postgres Driver
//!
//! # Query API
//!
//! Query API starts from the [`query`] function, it accepts a sql string, and an [`Executor`].
//!
//! An [`Executor`] is a flexible type that represent database connection. For more
//! information, see the [`executor`] module.
//!
//! ## The [`Query`] struct
//!
//! The [`query`] function returns a [`Query`] struct. The [`Query`] struct is a builder where
//! user can bind parameters and change query options.
//!
//! ### Encoding
//!
//! Having the builder pattern allows for easier multitype parameter instead of having it as
//! single type array. Parameter must implement [`Encode`], that is the value can be converted
//! into [`Encoded`] struct. The [`Encoded`] struct can be constructed with value that is one
//! of rust primitive type as [`ValueRef`], and an postgres [`Oid`].
//!
//! ### Query Option
//!
//! For now there is only one options that is persistency. All query are prepared,
//! cached, and reuse for subsequent query. But one can opt out of such behavior.
//!
//! ## Fetching
//!
//! Fetching define the output of a query. In [`Query`] struct, [`fetch_all`] method will
//! retrieve all rows as vector. [`fetch_one`] will retrieve one row, or return error otherwise.
//! [`fetch_optional`] will retrieve optionally retrieve one row.
//!
//! Fetch api require a type that can [`Decode`] a set of [`Row`]s.

mod common;
mod io;
mod net;
mod ext;

// Protocol
pub mod postgres;

// Codec
pub mod value;
pub mod encode;
pub mod column;
pub mod decode;

// Component
mod dberror;
pub mod statement;
pub mod row;

// Connection
pub mod options;
mod transport;
mod stream;
mod connection;

// Operation
pub mod protocol;
pub mod query;

mod error;


pub use self::options::PgOptions;
pub use self::connection::PgConnection;
pub use self::query::query;
pub use self::error::{Error, Result};

