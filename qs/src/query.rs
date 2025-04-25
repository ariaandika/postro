//! Query API types.
use std::marker::PhantomData;

use crate::encode::{Encode, Encoded};

mod ops;

mod portal;
mod fetch;
mod fetch_one;
mod fetch_all;
mod execute;
mod helpers;

pub use fetch::Fetch;
pub use fetch_one::FetchOne;
pub use fetch_all::FetchAll;
pub use execute::Execute;
pub use helpers::{StartupResponse, simple_query, startup};

/// Entrypoint of the query API.
pub fn query<'val, SQL, IO, R>(sql: SQL, io: IO) -> Query<'val, SQL, IO, R> {
    Query { sql, io, params: Vec::new(), _p: PhantomData }
}

/// Same as [`query`] but ignore the output.
pub fn query_row<'val, SQL, IO>(sql: SQL, io: IO) -> Query<'val, SQL, IO, ()> {
    Query { sql, io, params: Vec::new(), _p: PhantomData }
}

pub struct Query<'val, SQL, IO, R> {
    sql: SQL,
    io: IO,
    params: Vec<Encoded<'val>>,
    _p: PhantomData<R>,
}

impl<'val, SQL, IO, R> Query<'val, SQL, IO, R> {
    // /// Disable persistent prepared statement.
    // ///
    // /// This will use unnamed prepared statement under the hood,
    // /// which optimized for the case of executing a query only once and then discarding it.
    // ///
    // /// <https://www.postgresql.org/docs/current/protocol-overview.html#PROTOCOL-QUERY-CONCEPTS>
    // pub fn once(mut self) -> Self {
    //     self.persistent = false;
    //     self
    // }

    pub fn bind<V: Encode<'val>>(mut self, value: V) -> Self {
        self.params.push(value.encode());
        self
    }
}

impl<'val, SQL, IO, R> Query<'val, SQL, IO, R> {
    pub fn fetch(self) -> Fetch<'val, SQL, R, IO> {
        Fetch::new(self.sql, self.io, self.params, 0)
    }

    pub fn fetch_all(self) -> FetchAll<'val, SQL, R, IO> {
        FetchAll::new(self.sql, self.io, self.params)
    }

    pub fn fetch_one(self) -> FetchOne<'val, SQL, R, IO> {
        FetchOne::new(self.sql, self.io, self.params)
    }

    pub fn execute(self) -> Execute<'val, SQL, IO> {
        Execute::new(self.sql, self.io, self.params)
    }
}

