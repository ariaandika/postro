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
    postgres::{ProtocolError, backend},
    row::{FromRow, Row},
    transport::PgTransport,
};

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[project = FetchAllProject]
    pub struct Fetch<'sql, 'val, R, IO> {
        #[pin]
        phase: Phase<'sql, 'val, R, IO>,
        _p: PhantomData<R>,
    }
}

pin_project_lite::pin_project! {
    #[derive(Debug, Default)]
    #[project = PhaseProject]
    enum Phase<'sql, 'val, R, IO> {
        Portal {
            #[pin]
            portal: Portal<'sql, 'val, R, IO>,
        },
        PortalRecv {
            io: IO,
            cols: Option<Vec<ColumnInfo>>,
        },
        #[default]
        Invalid,
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
                    *phase = Phase::PortalRecv { io, cols: None };
                }
                PhaseProject::PortalRecv { io, cols } => {
                    use backend::BackendMessage::*;
                    loop {
                        match ready!(io.poll_recv(cx)?) {
                            RowDescription(rd) => {
                                cols.replace(ColumnInfo::decode_multi_vec(rd));
                            }
                            BindComplete(_) => {}
                            NoData(_) => {}
                            CommandComplete(_) => {}
                            ReadyForQuery(_) => break,
                            DataRow(dr) => {
                                let cols = cols.as_mut().expect("postgres didnt send RowDescription");
                                return Poll::Ready(Some(R::from_row(Row::new(cols, dr))));
                            }
                            f => {
                                let err =
                                    ProtocolError::unexpected_phase(f.msgtype(), "extended query");
                                *phase = Phase::Complete;
                                return Poll::Ready(Some(Err(err.into())));
                            }
                        }
                    }
                    *phase = Phase::Complete;
                }
                PhaseProject::Invalid => unreachable!(),
                PhaseProject::Complete => return Poll::Ready(None),
            }
        }
    }
}

