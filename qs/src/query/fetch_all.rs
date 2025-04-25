use futures_core::Stream;
use std::{
    mem,
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::Fetch;
use crate::{Result, encode::Encoded, row::FromRow, sql::Sql, transport::PgTransport};

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[project = FetchAllProject]
    pub struct FetchAll<'val, SQL, R, IO> {
        #[pin]
        fetch: Fetch<'val, SQL, R, IO>,
        output: Vec<R>,
    }
}

impl<'val, SQL, R, IO> FetchAll<'val, SQL, R, IO> {
    pub(crate) fn new(sql: SQL, io: IO, params: Vec<Encoded<'val>>) -> Self {
        Self {
            fetch: Fetch::new(sql, io, params, 0),
            output: vec![],
        }
    }
}

impl<SQL, R, IO> Future for FetchAll<'_, SQL, R, IO>
where
    SQL: Sql,
    R: FromRow,
    IO: PgTransport,
{
    type Output = Result<Vec<R>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        while let Some(r) = ready!(self.as_mut().project().fetch.poll_next(cx)?) {
            self.output.push(r);
        }
        Poll::Ready(Ok(mem::take(&mut self.output)))
    }
}

