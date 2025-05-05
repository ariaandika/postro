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
//! Database connection pooling:
//!
//! ```no_run
//! use postro::Pool;
//!
//! # async fn app() -> postro::Result<()> {
//! let mut pool = Pool::connect_env().await?;
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
//! # Ok(())
//! # }
//! # mod tokio { pub fn spawn<F>(_: F) -> F { todo!() } }
//! ```
//!
//! Begin a transaction:
//!
//! ```no_run
//! use postro::Connection;
//!
//! # async fn app() -> postro::Result<()> {
//! let mut conn = Connection::connect_env().await?;
//!
//! let mut tx = postro::begin(&mut conn).await?;
//!
//! let _res = postro::query::<_, _, (i32,String)>("INSERT INTO foo(id) VALUES($1)", &mut tx)
//!     .bind(14)
//!     .execute()
//!     .await?;
//!
//! // if this failed, `tx` will be droped and transaction is rolledback
//! fallible_operation()?;
//!
//! tx.commit().await?;
//! # Ok(())
//! # }
//! #
//! # fn fallible_operation() -> postro::Result<()> { todo!() }
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

pub mod types;

mod error;


#[doc(inline)]
pub use encode::Encode;
#[doc(inline)]
pub use row::{Row, FromRow, Decode, DecodeError};
pub use sql::SqlExt;

#[doc(inline)]
pub use executor::Executor;
#[doc(inline)]
pub use connection::{Connection, Config};
#[doc(inline)]
pub use pool::{Pool, PoolConfig};
#[doc(inline)]
pub use query::{query, execute, begin};
#[doc(inline)]
pub use error::{Error, Result};

#[cfg(feature = "macros")]
pub use postro_macros::FromRow;

