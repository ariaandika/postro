use futures_core::Stream;
use std::{
    mem,
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::FetchStream;
use crate::{Result, encode::Encoded, row::FromRow, sql::Sql, transport::PgTransport};

/// Returned [`fetch_all`][super::Query::fetch_all] future.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct FetchAll<'val, SQL, R, ExeFut, IO> {
    fetch: FetchStream<'val, SQL, R, ExeFut, IO>,
    output: Vec<R>,
}

impl<'val, SQL, R, ExeFut, IO> FetchAll<'val, SQL, R, ExeFut, IO> {
    pub(crate) fn new(sql: SQL, exe: ExeFut, params: Vec<Encoded<'val>>) -> Self {
        Self {
            fetch: FetchStream::new(sql, exe, params, 0),
            output: vec![],
        }
    }
}

impl<SQL, R, ExeFut, IO> Future for FetchAll<'_, SQL, R, ExeFut, IO>
where
    SQL: Sql + Unpin,
    R: FromRow + Unpin,
    ExeFut: Future<Output = Result<IO>> + Unpin,
    IO: PgTransport + Unpin,
{
    type Output = Result<Vec<R>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let me = self.get_mut();
        let f = &mut me.fetch;
        let output = &mut me.output;

        while let Some(r) = ready!(Pin::new(&mut *f).poll_next(cx)?) {
            output.push(r)
        }

        Poll::Ready(Ok(mem::take(output)))
    }
}

