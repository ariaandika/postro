//! Query API types.
use std::marker::PhantomData;

use crate::{
    Decode, FromRow, Result, Row,
    encode::{Encode, Encoded},
    executor::Executor,
    fetch::{Fetch, FetchCollect, FetchStream, StreamMap, command_complete},
    postgres::backend,
    row::{RowNotFound, RowResult},
    sql::Sql,
};

/// Entrypoint of the query API.
#[inline]
pub fn query<'val, SQL, Exe>(sql: SQL, exe: Exe) -> Query<'val, SQL, Exe, StreamRow<Row>> {
    Query { sql, exe, params: Vec::new(), _p: PhantomData }
}

/// Entrypoint of the query API.
#[inline]
pub fn query_as<'val, SQL, Exe, R>(sql: SQL, exe: Exe) -> Query<'val, SQL, Exe, StreamRow<R>> {
    Query { sql, exe, params: Vec::new(), _p: PhantomData }
}

/// Entrypoint of the query API.
#[inline]
pub fn query_scalar<'val, SQL, Exe, D>(sql: SQL, exe: Exe) -> Query<'val, SQL, Exe, StreamScalar<D>> {
    Query { sql, exe, params: Vec::new(), _p: PhantomData }
}

/// The query API.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Query<'val, SQL, Exe, M> {
    sql: SQL,
    exe: Exe,
    params: Vec<Encoded<'val>>,
    _p: PhantomData<M>,
}

impl<'val, SQL, Exe, M> Query<'val, SQL, Exe, M> {
    /// Bind query parameter.
    #[inline]
    pub fn bind<V: Encode<'val>>(mut self, value: V) -> Self {
        self.params.push(value.encode());
        self
    }
}

impl<'val, SQL, Exe, M> Query<'val, SQL, Exe, M> {
    /// Fetch rows using [`Stream`][futures_core::Stream] api.
    ///
    /// The returned `Stream` must be polled/awaited until completion,
    /// otherwise it will disturb subsequent query.
    ///
    /// Also if [`FromRow`][crate::FromRow] implementation returns error,
    /// stream is suspended.
    #[inline]
    pub fn fetch(self) -> FetchStream<'val, SQL, Exe::Future, Exe::Transport, M>
    where
        Exe: Executor,
        M: StreamMap,
    {
        FetchStream::new(self.sql, self.exe.connection(), self.params, 0)
    }

    /// Fetch all rows into [`Vec`].
    #[inline]
    pub fn fetch_all(self) -> Fetch<'val, SQL, Exe::Future, Exe::Transport, M, CollectAll<M::Output>>
    where
        Exe: Executor,
        M: StreamMap,
    {
        Fetch::new(
            self.sql,
            self.exe.connection(),
            self.params,
            CollectAll(Vec::new()),
            0,
        )
    }

    /// Fetch one row.
    #[inline]
    pub fn fetch_one(self) -> Fetch<'val, SQL, Exe::Future, Exe::Transport, M, CollectOne<M::Output>>
    where
        Exe: Executor,
        M: StreamMap,
    {
        Fetch::new(
            self.sql,
            self.exe.connection(),
            self.params,
            CollectOne(None),
            1,
        )
    }

    /// Optionally fetch one row.
    #[inline]
    pub fn fetch_optional(self) -> Fetch<'val, SQL, Exe::Future, Exe::Transport, M, CollectOpt<M::Output>>
    where
        Exe: Executor,
        M: StreamMap,
    {
        Fetch::new(
            self.sql,
            self.exe.connection(),
            self.params,
            CollectOpt(None),
            1,
        )
    }

    /// Execute statement and return number of rows affected.
    #[inline]
    pub fn execute(self) -> Fetch<'val, SQL, Exe::Future, Exe::Transport, M, CollectCmd>
    where
        Exe: Executor,
    {
        Fetch::new(self.sql, self.exe.connection(), self.params, CollectCmd, 0)
    }
}

impl<'val, SQL, Exe, M> IntoFuture for Query<'val, SQL, Exe, M>
where
    SQL: Sql + Unpin,
    Exe: Executor + Unpin,
    M: StreamMap<Output = Row> + Unpin,
{
    type Output = Result<RowResult>;

    type IntoFuture = Fetch<'val, SQL, Exe::Future, Exe::Transport, M, CollectCmd>;

    #[inline]
    fn into_future(self) -> Self::IntoFuture {
        self.execute()
    }
}


// ===== Stream Adapters =====

pub struct StreamRow<R>(PhantomData<R>);

pub struct StreamScalar<D>(PhantomData<D>);

impl<R> StreamMap for StreamRow<R>
where
    R: FromRow,
{
    type Output = R;

    #[inline]
    fn map(row: Row) -> Result<Self::Output> {
        R::from_row(row).map_err(Into::into)
    }
}

impl<D> StreamMap for StreamScalar<D>
where
    D: Decode,
{
    type Output = D;

    #[inline]
    fn map(row: Row) -> Result<Self::Output> {
        match <(D,)>::from_row(row) {
            Ok(ok) => Ok(ok.0),
            Err(err) => Err(err.into()),
        }
    }
}

// ===== Fetch Adapters =====

/// [`FetchCollect`] adapter used by [`fetch_all`][Query::fetch_all].
#[derive(Debug)]
pub struct CollectAll<R>(pub Vec<R>);

/// [`FetchCollect`] adapter used by [`fetch_one`][Query::fetch_one].
#[derive(Debug)]
pub struct CollectOne<R>(pub Option<R>);

/// [`FetchCollect`] adapter used by [`fetch_optional`][Query::fetch_optional].
#[derive(Debug)]
pub struct CollectOpt<R>(pub Option<R>);

/// [`FetchCollect`] adapter used by [`execute`][Query::execute].
#[derive(Debug)]
pub struct CollectCmd;

impl<R> FetchCollect<R> for CollectAll<R> {
    type Output = Vec<R>;

    #[inline]
    fn value(&mut self, input: R) {
        self.0.push(input);
    }

    #[inline]
    fn finish(&mut self, _: Option<backend::CommandComplete>) -> Result<Self::Output> {
        Ok(std::mem::take(&mut self.0))
    }
}

impl<R> FetchCollect<R> for CollectOpt<R> {
    type Output = Option<R>;

    #[inline]
    fn value(&mut self, input: R) {
        self.0 = Some(input);
    }

    #[inline]
    fn finish(&mut self, _: Option<backend::CommandComplete>) -> Result<Self::Output> {
        Ok(self.0.take())
    }
}

impl<R> FetchCollect<R> for CollectOne<R> {
    type Output = R;

    #[inline]
    fn value(&mut self, input: R) {
        self.0 = Some(input);
    }

    #[inline]
    fn finish(&mut self, _: Option<backend::CommandComplete>) -> Result<Self::Output> {
        match self.0.take() {
            Some(ok) => Ok(ok),
            None => Err(RowNotFound.into()),
        }
    }
}

impl FetchCollect<Row> for CollectCmd {
    type Output = RowResult;

    #[inline]
    fn value(&mut self, _: Row) {}

    #[inline]
    fn finish(&mut self, cmd: Option<backend::CommandComplete>) -> Result<Self::Output> {
        Ok(RowResult {
            rows_affected: cmd.map(command_complete).expect("only PortalSuspended"),
        })
    }
}

