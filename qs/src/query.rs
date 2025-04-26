//! Query API types.
use std::marker::PhantomData;

use crate::{encode::{Encode, Encoded}, executor::Executor};

mod ops;

mod portal;
mod fetch_stream;
mod fetch_one;
mod fetch_all;
mod execute;
mod helpers;

pub use fetch_stream::Fetch;
pub use fetch_one::FetchOne;
pub use fetch_all::FetchAll;
pub use execute::Execute;
pub use helpers::{StartupResponse, simple_query, startup};

/// Entrypoint of the query API.
pub fn query<'val, SQL, Exe, R>(sql: SQL, exe: Exe) -> Query<'val, SQL, Exe, R> {
    Query { sql, exe, params: Vec::new(), _p: PhantomData }
}

/// Same as [`query`] but ignore the output.
pub fn execute<'val, SQL, Exe>(sql: SQL, exe: Exe) -> Query<'val, SQL, Exe, ()> {
    Query { sql, exe, params: Vec::new(), _p: PhantomData }
}

pub struct Query<'val, SQL, Exe, R> {
    sql: SQL,
    exe: Exe,
    params: Vec<Encoded<'val>>,
    _p: PhantomData<R>,
}

impl<'val, SQL, Exe, R> Query<'val, SQL, Exe, R> {
    pub fn bind<V: Encode<'val>>(mut self, value: V) -> Self {
        self.params.push(value.encode());
        self
    }
}

impl<'val, SQL, Exe, R> Query<'val, SQL, Exe, R>
where
    Exe: Executor,
{
    /// Fetch rows using [`Stream`][futures_core::Stream] api.
    pub fn fetch(self) -> Fetch<'val, SQL, R, Exe::Future, Exe::Transport> {
        Fetch::new(self.sql, self.exe.connection(), self.params, 0)
    }

    pub fn fetch_all(self) -> FetchAll<'val, SQL, R, Exe::Future, Exe::Transport> {
        FetchAll::new(self.sql, self.exe.connection(), self.params)
    }

    pub fn fetch_one(self) -> FetchOne<'val, SQL, R, Exe::Future, Exe::Transport> {
        FetchOne::new(self.sql, self.exe.connection(), self.params)
    }

    pub fn execute(self) -> Execute<'val, SQL, Exe::Future, Exe::Transport> {
        Execute::new(self.sql, self.exe.connection(), self.params)
    }
}

