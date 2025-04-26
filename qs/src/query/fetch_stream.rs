use futures_core::Stream;
use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::portal::Portal;
use crate::{
    Result,
    column::ColumnInfo,
    encode::Encoded,
    postgres::backend,
    row::{FromRow, Row},
    sql::Sql,
    transport::PgTransport,
};

#[derive(Debug)]
pub struct FetchStream<'val, SQL, R, ExeFut, IO> {
    phase: Phase<'val, SQL, ExeFut, IO>,
    _p: PhantomData<R>,
}

#[derive(Debug)]
enum Phase<'val, SQL, ExeFut, IO> {
    Portal { portal: Portal<'val, SQL, ExeFut, IO> },
    BindComplete { io: Option<IO> },
    RowDescription { io: Option<IO> },
    DataRow {
        io: Option<IO>,
        cols: Vec<ColumnInfo>,
    },
    ReadyForQuery { io: IO },
    Complete,
}

impl<'val, SQL, R, ExeFut, IO> FetchStream<'val, SQL, R, ExeFut, IO> {
    pub(crate) fn new(
        sql: SQL,
        exe: ExeFut,
        params: Vec<Encoded<'val>>,
        max_row: u32,
    ) -> Self {
        Self {
            phase: Phase::Portal {
                portal: Portal::new(sql, exe, params, max_row),
            },
            _p: PhantomData,
        }
    }
}

impl<SQL, R, ExeFut, IO> Stream for FetchStream<'_, SQL, R, ExeFut, IO>
where
    SQL: Sql,
    R: FromRow,
    ExeFut: Future<Output = IO>,
    IO: PgTransport,
{
    type Item = Result<R>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // SAFETY: `self` never move
        let me = unsafe { self.get_unchecked_mut() };
        let phase = &mut me.phase;

        loop {
            match &mut *phase {
                Phase::Portal { portal } => {
                    // SAFETY: `me` never move
                    let portal = unsafe { Pin::new_unchecked(portal) };
                    let io = ready!(portal.poll(cx)?);
                    *phase = Phase::BindComplete { io: Some(io) };
                },
                Phase::BindComplete { io } => {
                    ready!(io.as_mut().unwrap().poll_recv::<backend::BindComplete>(cx)?);
                    *phase = Phase::RowDescription { io: io.take() };
                },
                Phase::RowDescription { io } => {
                    // `NoData` is invalid, because `Fetch` expect row to be returned
                    let rd = ready!(io.as_mut().unwrap().poll_recv::<backend::RowDescription>(cx)?);
                    let cols = ColumnInfo::decode_multi_vec(rd)?;
                    *phase = Phase::DataRow { io: io.take(), cols };
                },
                Phase::DataRow { io, cols } => {
                    use backend::BackendMessage::*;
                    match ready!(io.as_mut().unwrap().poll_recv(cx)?) {
                        DataRow(dr) => {
                            return Poll::Ready(Some(R::from_row(Row::new(cols, dr)).map_err(Into::into)));
                        }

                        // `Execute` phase terminations:
                        CommandComplete(_) | PortalSuspended(_) | EmptyQueryResponse(_) => {},
                        f => {
                            let err = f.unexpected("fetching rows");
                            *phase = Phase::Complete;
                            return Poll::Ready(Some(Err(err.into())));
                        }
                    }
                    *phase = Phase::ReadyForQuery { io: io.take().unwrap() };
                },
                Phase::ReadyForQuery { io } => {
                    ready!(io.poll_recv::<backend::ReadyForQuery>(cx)?);
                    *phase = Phase::Complete;
                },
                Phase::Complete => return Poll::Ready(None),
            }
        }
    }
}

