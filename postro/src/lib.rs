//! Postgres Driver
//!
//! # Examples
//!
//! Single connection:
//!
//! ```no_run
//! use postro::Connection;
//!
//! # async fn app() -> postro::Result<()> {
//! let mut conn = Connection::connect_env().await?;
//!
//! let res = postro::query::<_, _, (i32,String)>("SELECT 420,$1", &mut conn)
//!     .bind("Foo")
//!     .fetch_one()
//!     .await?;
//!
//! assert_eq!(res.0,420);
//! assert_eq!(res.1.as_str(),"Foo");
//! # Ok(())
//! # }
//! ```
//!
//! Database Pooling:
//!
//! ```no_run
//! use postro::Pool;
//!
//! # async fn app() -> postro::Result<()> {
//! let mut pool = Pool::connect_env().await?;
//!
//! postro::execute("CREATE TEMP TABLE foo(id int)", &mut pool)
//!     .execute()
//!     .await?;
//!
//! let mut handles = vec![];
//!
//! for i in 0..14 {
//!     let mut pool = pool.clone();
//!     let t = tokio::spawn(async move {
//!         postro::execute("INSERT INTO foo(id) VALUES($1)", &mut pool)
//!             .bind(i)
//!             .execute()
//!             .await
//!     });
//!     handles.push(t);
//! }
//!
//! for h in handles {
//!     h.await.unwrap();
//! }
//!
//! let foos = postro::query::<_, _, (i32,)>("SELECT * FROM foo", &mut pool)
//!     .fetch_all()
//!     .await?;
//!
//! assert_eq!(foos.len(), 14);
//!
//! # Ok(())
//! # }
//! # mod tokio { pub fn spawn<F>(_: F) -> F { todo!() } }
//! ```

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
pub use row::{Row, FromRow, FromColumn, DecodeError};
pub use sql::SqlExt;

pub use executor::Executor;
pub use connection::{Connection, Config};
pub use pool::{Pool, PoolConfig};
#[doc(inline)]
pub use query::{query, execute, begin};
pub use error::{Error, Result};

#[cfg(feature = "macros")]
pub use postro_macros::FromRow;

