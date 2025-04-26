use futures_core::Stream;
use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::Fetch;
use crate::{Result, encode::Encoded, row::FromRow, sql::Sql, transport::PgTransport};

#[derive(Debug)]
pub struct FetchOne<'val, SQL, R, ExeMut, IO> {
    fetch: Fetch<'val, SQL, R, ExeMut, IO>,
    row: Option<R>,
    complete: bool,
}

impl<'val, SQL, R, ExeMut, IO> FetchOne<'val, SQL, R, ExeMut, IO> {
    pub(crate) fn new(sql: SQL, exe: ExeMut, params: Vec<Encoded<'val>>) -> Self {
        Self {
            fetch: Fetch::new(sql, exe, params, 1),
            row: None,
            complete: false,
        }
    }
}

impl<SQL, R, ExeFut, IO> Future for FetchOne<'_, SQL, R, ExeFut, IO>
where
    SQL: Sql,
    R: FromRow,
    ExeFut: Future<Output = IO>,
    IO: PgTransport,
{
    type Output = Result<R>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if self.complete {
            panic!("`poll` after complete");
        }

        // SAFETY: `self` never move
        let me = unsafe { self.get_unchecked_mut() };

        loop {
            // SAFETY: `me` never move
            let f = unsafe { Pin::new_unchecked(&mut me.fetch) };
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

