use futures_core::Stream;
use std::{
    mem,
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::Fetch;
use crate::{Result, encode::Encoded, row::FromRow, transport::PgTransport};

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[project = FetchAllProject]
    pub struct FetchAll<'sql, 'val, R, IO> {
        #[pin]
        fetch: Fetch<'sql, 'val, R, IO>,
        output: Vec<R>,
    }
}

impl<'sql, 'val, R, IO> FetchAll<'sql, 'val, R, IO> {
    pub(crate) fn new(sql: &'sql str, io: IO, params: Vec<Encoded<'val>>, persistent: bool) -> Self {
        Self {
            fetch: Fetch::new(sql, io, params, 0, persistent),
            output: vec![],
        }
    }
}

impl<R, IO> Future for FetchAll<'_, '_, R, IO>
where
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

