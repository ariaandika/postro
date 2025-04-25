use futures_core::Stream;
use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::Fetch;
use crate::{Result, encode::Encoded, row::FromRow, sql::Sql, transport::PgTransport};

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[project = FetchOneProject]
    pub struct FetchOne<'val, SQL, R, IO> {
        #[pin]
        fetch: Fetch<'val, SQL, R, IO>,
    }
}

impl<'val, SQL, R, IO> FetchOne<'val, SQL, R, IO> {
    pub(crate) fn new(sql: SQL, io: IO, params: Vec<Encoded<'val>>) -> Self {
        Self {
            fetch: Fetch::new(sql, io, params, 1),
        }
    }
}

impl<SQL, R, IO> Future for FetchOne<'_, SQL, R, IO>
where
    SQL: Sql,
    R: FromRow,
    IO: PgTransport,
{
    type Output = Result<R>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let Some(r) = ready!(self.as_mut().project().fetch.poll_next(cx)?) else {
            todo!()
        };
        // `PortalSuspended`
        ready!(self.as_mut().project().fetch.poll_next(cx)?);
        Poll::Ready(Ok(r))
    }
}

