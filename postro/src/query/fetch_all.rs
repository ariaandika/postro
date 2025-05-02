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
    SQL: Sql,
    R: FromRow,
    ExeFut: Future<Output = Result<IO>>,
    IO: PgTransport,
{
    type Output = Result<Vec<R>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        // SAFETY: `self` never move
        let me = unsafe { self.get_unchecked_mut() };
        let f = &mut me.fetch;
        let output = &mut me.output;

        // SAFETY: `me` never move
        while let Some(r) = ready!(unsafe { Pin::new_unchecked(&mut *f) }.poll_next(cx)?) {
            output.push(r)
        }

        Poll::Ready(Ok(mem::take(output)))
    }
}

