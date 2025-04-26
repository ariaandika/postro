use futures_core::Stream;
use std::{
    mem,
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::Fetch;
use crate::{Result, encode::Encoded, row::FromRow, sql::Sql, transport::PgTransport};

#[derive(Debug)]
pub struct FetchAll<'val, SQL, R, ExeFut, IO> {
    fetch: Fetch<'val, SQL, R, ExeFut, IO>,
    output: Vec<R>,
}

impl<'val, SQL, R, ExeFut, IO> FetchAll<'val, SQL, R, ExeFut, IO> {
    pub(crate) fn new(sql: SQL, exe: ExeFut, params: Vec<Encoded<'val>>) -> Self {
        Self {
            fetch: Fetch::new(sql, exe, params, 0),
            output: vec![],
        }
    }
}

impl<SQL, R, ExeFut, IO> Future for FetchAll<'_, SQL, R, ExeFut, IO>
where
    SQL: Sql,
    R: FromRow,
    ExeFut: Future<Output = IO>,
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

        return Poll::Ready(Ok(mem::take(output)));
    }
}

