use futures_core::Stream;
use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::Fetch;
use crate::{Result, encode::Encoded, row::FromRow, transport::PgTransport};

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[project = FetchOneProject]
    pub struct FetchOne<'sql, 'val, R, IO> {
        #[pin]
        fetch: Fetch<'sql, 'val, R, IO>,
    }
}

impl<'sql, 'val, R, IO> FetchOne<'sql, 'val, R, IO> {
    pub fn new(sql: &'sql str, io: IO, params: Vec<Encoded<'val>>, persistent: bool) -> Self {
        Self {
            fetch: Fetch::new(sql, io, params, 1, persistent),
        }
    }
}

impl<R, IO> Future for FetchOne<'_, '_, R, IO>
where
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

