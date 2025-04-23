use std::{
    mem,
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::ops::{self, PrepareData};
use crate::{Result, encode::Encoded, postgres::backend, transport::PgTransport};

pin_project_lite::pin_project! {
    /// Prepare a statement and bind a portal.
    ///
    /// Caller must ready to receive subsequent messages explained in [`portal`](super::ops::portal)
    #[derive(Debug)]
    #[project = PortalProject]
    pub struct Portal<'sql, 'val, IO> {
        sql: &'sql str,
        io: Option<IO>,
        phase: Phase,
        params: Vec<Encoded<'val>>,
        max_row: u32,
        persistent: bool,
    }
}

impl<'sql, 'val, IO> Portal<'sql, 'val, IO> {
    /// Create new [`Portal`] future.
    pub(crate) fn new(
        sql: &'sql str,
        io: IO,
        params: Vec<Encoded<'val>>,
        max_row: u32,
        persistent: bool,
    ) -> Self {
        Self {
            sql,
            io: Some(io),
            phase: Phase::Prepare,
            params,
            max_row,
            persistent,
        }
    }
}

#[derive(Debug, Default)]
enum Phase {
    Prepare,
    PrepareFlush(PrepareData),
    PrepareComplete(PrepareData),
    Portal(PrepareData),
    PortalFlush,
    #[default]
    Invalid,
    Complete,
}

impl<IO> Future for Portal<'_, '_, IO>
where
    IO: PgTransport,
{
    type Output = Result<IO>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let PortalProject {
            sql,
            io: self_io,
            phase,
            params,
            max_row,
            persistent,
        } = self.as_mut().project();

        let io = self_io.as_mut().expect("foo poll after complete");

        loop {
            match &mut *phase {
                Phase::Prepare => {
                    let data = ops::prepare(sql, params, *persistent, &mut *io);
                    *phase = match data.cache_hit {
                        true => Phase::Portal(data),
                        false => Phase::PrepareFlush(data),
                    };
                }
                Phase::PrepareFlush(_) => {
                    ready!(io.poll_flush(cx)?);
                    let Phase::PrepareFlush(data) = mem::take(phase) else {
                        unreachable!()
                    };
                    *phase = Phase::PrepareComplete(data);
                }
                Phase::PrepareComplete(_) => {
                    ready!(io.poll_recv::<backend::ParseComplete>(cx)?);
                    let Phase::PrepareComplete(data) = mem::take(phase) else {
                        unreachable!()
                    };
                    io.add_stmt(data.sqlid, data.stmt.clone());
                    *phase = Phase::Portal(data);
                }
                Phase::Portal(data) => {
                    data.max_row = *max_row;
                    ops::portal(data, params, &mut *io);
                    *phase = Phase::PortalFlush;
                }
                Phase::PortalFlush => {
                    ready!(io.poll_flush(cx)?);
                    *phase = Phase::Complete;
                    return Poll::Ready(Ok(self_io.take().expect("foo poll after complete")));
                }
                Phase::Invalid => unreachable!(),
                Phase::Complete => panic!("`poll` after complete"),
            }
        }
    }
}

