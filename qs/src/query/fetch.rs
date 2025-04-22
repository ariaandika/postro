use futures_core::Stream;
use std::{
    marker::PhantomData, pin::Pin, task::{ready, Context, Poll}
};

use super::portal::Portal;
use crate::{
    Result,
    column::ColumnInfo,
    encode::Encoded,
    postgres::{ProtocolError, backend},
    row::{FromRow, Row},
    transport::PgTransport,
};

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[project = FetchAllProject]
    pub struct Fetch<'sql, 'val, R, IO> {
        #[pin]
        phase: Phase<'sql, 'val, IO>,
        _p: PhantomData<R>,
    }
}

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[project = PhaseProject]
    enum Phase<'sql, 'val, IO> {
        Portal { #[pin] portal: Portal<'sql, 'val, IO> },
        BindComplete { io: Option<IO> },
        RowDescription { io: Option<IO> },
        DataRow {
            io: Option<IO>,
            cols: Vec<ColumnInfo>,
        },
        ReadyForQuery { io: IO },
        Complete,
    }
}

impl<'sql, 'val, R, IO> Fetch<'sql, 'val, R, IO> {
    pub fn new(sql: &'sql str, io: IO, params: Vec<Encoded<'val>>, persistent: bool) -> Self {
        Self {
            phase: Phase::Portal {
                portal: Portal::new(sql, io, params, persistent),
            },
            _p: PhantomData,
        }
    }
}

impl<R, IO> Stream for Fetch<'_, '_, R, IO>
where
    R: FromRow,
    IO: PgTransport,
{
    type Item = Result<R>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let FetchAllProject { mut phase, _p } = self.as_mut().project();

        loop {
            match phase.as_mut().project() {
                PhaseProject::Portal { portal } => {
                    let io = ready!(portal.poll(cx)?);
                    *phase = Phase::BindComplete { io: Some(io) };
                },
                PhaseProject::BindComplete { io } => {
                    ready!(io.as_mut().unwrap().poll_recv::<backend::BindComplete>(cx)?);
                    *phase = Phase::RowDescription { io: io.take() };
                },
                PhaseProject::RowDescription { io } => {
                    // `NoData` is invalid, because `Fetch` expect row to be returned
                    let rd = ready!(io.as_mut().unwrap().poll_recv::<backend::RowDescription>(cx)?);
                    let cols = ColumnInfo::decode_multi_vec(rd);
                    *phase = Phase::DataRow { io: io.take(), cols };
                },
                PhaseProject::DataRow { io, cols } => {
                    use backend::BackendMessage::*;
                    loop {
                        match ready!(io.as_mut().unwrap().poll_recv(cx)?) {
                            DataRow(dr) => {
                                return Poll::Ready(Some(R::from_row(Row::new(cols, dr))));
                            }

                            // `Execute` phase is terminations:
                            CommandComplete(_) | PortalSuspended(_) | EmptyQueryResponse(_) => break,
                            f => {
                                let err = ProtocolError::unexpected_phase(f.msgtype(), "row execution");
                                *phase = Phase::Complete;
                                return Poll::Ready(Some(Err(err.into())));
                            }
                        }
                    }
                    *phase = Phase::ReadyForQuery { io: io.take().unwrap() };
                },
                PhaseProject::ReadyForQuery { io } => {
                    ready!(io.poll_recv::<backend::ReadyForQuery>(cx)?);
                    *phase = Phase::Complete;
                },
                PhaseProject::Complete => return Poll::Ready(None),
            }
        }
    }
}

