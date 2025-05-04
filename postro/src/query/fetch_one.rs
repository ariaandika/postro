use futures_core::Stream;
use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::FetchStream;
use crate::{Result, encode::Encoded, row::FromRow, sql::Sql, transport::PgTransport};

/// Returned [`fetch_one`][super::Query::fetch_one] future.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct FetchOne<'val, SQL, R, ExeMut, IO> {
    fetch: FetchStream<'val, SQL, R, ExeMut, IO>,
    row: Option<R>,
    complete: bool,
}

impl<'val, SQL, R, ExeMut, IO> FetchOne<'val, SQL, R, ExeMut, IO> {
    pub(crate) fn new(sql: SQL, exe: ExeMut, params: Vec<Encoded<'val>>) -> Self {
        Self {
            fetch: FetchStream::new(sql, exe, params, 1),
            row: None,
            complete: false,
        }
    }
}

impl<SQL, R, ExeFut, IO> Future for FetchOne<'_, SQL, R, ExeFut, IO>
where
    SQL: Sql + Unpin,
    R: FromRow + Unpin,
    ExeFut: Future<Output = Result<IO>> + Unpin,
    IO: PgTransport + Unpin,
{
    type Output = Result<R>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if self.complete {
            panic!("`poll` after complete");
        }

        let me = self.get_mut();

        loop {
            let f = Pin::new(&mut me.fetch);
            let row = &mut me.row;
            let complete = &mut me.complete;

            match &mut *row {
                None => {
                    let Some(r) = ready!(f.poll_next(cx)?) else {
                        todo!("Error NoRowFound")
                    };
                    assert!(row.replace(r).is_none());
                },
                Some(_) => {
                    // `PortalSuspended`
                    assert!(ready!(f.poll_next(cx)?).is_none());
                    *complete = true;
                    return Poll::Ready(Ok(row.take().unwrap()));
                },
            }
        }
    }
}

