use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::portal::Portal;
use crate::{
    Result,
    column::ColumnInfo,
    encode::Encoded,
    postgres::{ProtocolError, backend},
    transport::PgTransport,
};

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[project = ExecuteProject]
    pub struct Execute<'sql, 'val, IO> {
        #[pin]
        phase: Phase<'sql, 'val, IO>,
    }
}

pin_project_lite::pin_project! {
    #[derive(Debug, Default)]
    #[project = PhaseProject]
    enum Phase<'sql, 'val, IO> {
        Portal {
            #[pin]
            portal: Portal<'sql, 'val, IO>,
        },
        Execute {
            io: Option<IO>,
            cols: Option<Vec<ColumnInfo>>,
        },
        ReadyForQuery {
            io: IO,
        },
        #[default]
        Invalid,
        Complete,
    }
}

impl<'sql, 'val, IO> Execute<'sql, 'val, IO> {
    pub fn new(sql: &'sql str, io: IO, params: Vec<Encoded<'val>>, persistent: bool) -> Self {
        Self {
            phase: Phase::Portal {
                portal: Portal::new(sql, io, params, persistent),
            },
        }
    }
}

impl<IO> Future for Execute<'_, '_, IO>
where
    IO: PgTransport,
{
    type Output = Result<i32>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ExecuteProject { mut phase, } = self.as_mut().project();
        loop {
            match phase.as_mut().project() {
                PhaseProject::Portal { portal } => {
                    let io = ready!(portal.poll(cx)?);
                    *phase = Phase::Execute { io: Some(io), cols: None };
                }
                PhaseProject::Execute { io, cols } => {
                    use backend::BackendMessage::*;
                    loop {
                        match ready!(io.as_mut().unwrap().poll_recv(cx)?) {
                            RowDescription(rd) => {
                                cols.replace(ColumnInfo::decode_multi_vec(rd));
                            }
                            BindComplete(_) => {}
                            NoData(_) => {}
                            CommandComplete(_) => {}
                            DataRow(_) => {
                                // let cols = cols.as_mut().expect("postgres didnt send RowDescription");
                                // let row = R::from_row(Row::new(cols, dr))?;
                                let io = io.take().unwrap();
                                *phase = Phase::ReadyForQuery { io, /* row: Some(row) */ };
                                break
                            }
                            f => {
                                let err = ProtocolError::unexpected_phase(f.msgtype(), "extended query");
                                *phase = Phase::Complete;
                                return Poll::Ready(Err(err.into()));
                            }
                        }
                    }
                },
                PhaseProject::ReadyForQuery { io, /* row */ } => {
                    use backend::BackendMessage::*;
                    loop {
                        match ready!(io.poll_recv(cx)?) {
                            ReadyForQuery(_) => break,
                            f => {
                                let err = ProtocolError::unexpected_phase(f.msgtype(), "extended query");
                                *phase = Phase::Complete;
                                return Poll::Ready(Err(err.into()));
                            }
                        }
                    }
                    // return Poll::Ready(Ok(row.take().expect("`poll` after complete")));
                    return Poll::Ready(Ok(420));
                },
                PhaseProject::Invalid => unreachable!(),
                PhaseProject::Complete => panic!("`poll` after complete"),
            }
        }
    }
}

