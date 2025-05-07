//! Query API types.
use std::marker::PhantomData;

use crate::{
    Result, Row,
    encode::{Encode, Encoded},
    executor::Executor,
    fetch::{Execute, FetchAll, FetchOne, FetchOptional, FetchStream},
    row::RowResult,
    sql::Sql,
};

/// Entrypoint of the query API.
pub fn query<'val, SQL, Exe, R>(sql: SQL, exe: Exe) -> Query<'val, SQL, Exe, R> {
    Query { sql, exe, params: Vec::new(), _p: PhantomData }
}

/// Same as [`query`] with [`Row`] as the output.
pub fn query_row<'val, SQL, Exe>(sql: SQL, exe: Exe) -> Query<'val, SQL, Exe, Row> {
    Query { sql, exe, params: Vec::new(), _p: PhantomData }
}

/// Same as [`query`] with [`Row`] as the output.
pub fn execute<'val, SQL, Exe>(sql: SQL, exe: Exe) -> Query<'val, SQL, Exe, Row> {
    Query { sql, exe, params: Vec::new(), _p: PhantomData }
}

/// The query API.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Query<'val, SQL, Exe, R> {
    sql: SQL,
    exe: Exe,
    params: Vec<Encoded<'val>>,
    _p: PhantomData<R>,
}

impl<'val, SQL, Exe, R> Query<'val, SQL, Exe, R> {
    /// Bind query parameter.
    pub fn bind<V: Encode<'val>>(mut self, value: V) -> Self {
        self.params.push(value.encode());
        self
    }
}

impl<'val, SQL, Exe, R> Query<'val, SQL, Exe, R> {
    /// Fetch rows using [`Stream`][futures_core::Stream] api.
    ///
    /// The returned `Stream` must be polled/awaited until completion,
    /// otherwise it will disturb subsequent query.
    ///
    /// Also if [`FromRow`][crate::FromRow] implementation returns error,
    /// stream is suspended.
    pub fn fetch(self) -> FetchStream<'val, SQL, Exe::Future, Exe::Transport, R>
    where
        Exe: Executor,
    {
        FetchStream::new(self.sql, self.exe.connection(), self.params, 0)
    }

    /// Fetch all rows into [`Vec`].
    pub fn fetch_all(self) -> FetchAll<'val, SQL, Exe::Future, Exe::Transport, R>
    where
        Exe: Executor,
    {
        FetchAll::new(self.sql, self.exe.connection(), self.params)
    }

    /// Fetch one row.
    pub fn fetch_one(self) -> FetchOne<'val, SQL, Exe::Future, Exe::Transport, R>
    where
        Exe: Executor,
    {
        FetchOne::new(self.sql, self.exe.connection(), self.params)
    }

    /// Optionally fetch one row.
    pub fn fetch_optional(self) -> FetchOptional<'val, SQL, Exe::Future, Exe::Transport, R>
    where
        Exe: Executor,
    {
        FetchOptional::new(self.sql, self.exe.connection(), self.params)
    }

    /// Execute statement and return number of rows affected.
    pub fn execute(self) -> Execute<'val, SQL, Exe::Future, Exe::Transport>
    where
        Exe: Executor,
    {
        Execute::new(self.sql, self.exe.connection(), self.params)
    }
}

impl<'val, SQL, Exe, R> IntoFuture for Query<'val, SQL, Exe, R>
where
    SQL: Sql + Unpin,
    Exe: Executor + Unpin,
{
    type Output = Result<RowResult>;

    type IntoFuture = Execute<'val, SQL, Exe::Future, Exe::Transport>;

    fn into_future(self) -> Self::IntoFuture {
        self.execute()
    }
}

