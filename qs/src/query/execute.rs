use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::{ops, portal::Portal};
use crate::{Result, encode::Encoded, postgres::backend, sql::Sql, transport::PgTransport};

/// Returned [`execute`][super::Query::execute] future.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Execute<'val, SQL, ExeFut, IO> {
    phase: Phase<'val, SQL, ExeFut, IO>,
}

#[derive(Debug)]
enum Phase<'val, SQL, ExeFut, IO> {
    Portal {
        portal: Portal<'val, SQL, ExeFut, IO>,
    },
    BindComplete { io: Option<IO> },
    NoData { io: Option<IO> },
    Execute { io: Option<IO> },
    ReadyForQuery { io: IO, row_info: u64 },
    Complete,
}

impl<'val, SQL, ExeFut, IO> Execute<'val, SQL, ExeFut, IO> {
    pub(crate) fn new(sql: SQL, exe: ExeFut, params: Vec<Encoded<'val>>) -> Self {
        Self {
            phase: Phase::Portal {
                portal: Portal::new(sql, exe, params, 0),
            },
        }
    }
}

impl<SQL, ExeFut, IO> Future for Execute<'_, SQL, ExeFut, IO>
where
    SQL: Sql,
    ExeFut: Future<Output = IO>,
    IO: PgTransport,
{
    type Output = Result<u64>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
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
                    *phase = Phase::NoData { io: io.take() };
                },
                Phase::NoData { io } => {
                    ready!(io.as_mut().unwrap().poll_recv::<backend::NoData>(cx)?);
                    *phase = Phase::Execute { io: io.take() };
                }
                Phase::Execute { io } => {
                    let cmd = ready!(io.as_mut().unwrap().poll_recv::<backend::CommandComplete>(cx)?);
                    let row_info = ops::command_complete(cmd);
                    *phase = Phase::ReadyForQuery { io: io.take().unwrap(), row_info };
                },
                Phase::ReadyForQuery { io, row_info } => {
                    ready!(io.poll_recv::<backend::ReadyForQuery>(cx)?);
                    let row_info = *row_info;
                    *phase = Phase::Complete;
                    return Poll::Ready(Ok(row_info));
                },
                Phase::Complete => panic!("`poll` after complete"),
            }
        }
    }
}

