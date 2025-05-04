use std::{
    mem,
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::ops::{self, PrepareData};
use crate::{Result, encode::Encoded, postgres::backend, sql::Sql, transport::PgTransport};

/// Prepare a statement and bind a portal.
///
/// Caller must ready to receive subsequent messages explained in [`portal`](super::ops::portal).
#[derive(Debug)]
pub struct Portal<'val, SQL, ExeFut, IO> {
    sql: SQL,
    io: Option<IO>,
    phase: Phase<ExeFut>,
    params: Vec<Encoded<'val>>,
    max_row: u32,
}

#[derive(Debug, Default)]
enum Phase<ExeFut> {
    Connect { f: ExeFut },
    Prepare,
    PrepareFlush(PrepareData),
    PrepareComplete(PrepareData),
    Portal(PrepareData),
    PortalFlush,
    #[default]
    Invalid,
    Complete,
}

impl<'val, SQL, ExeFut, IO> Portal<'val, SQL, ExeFut, IO> {
    /// Create new [`Portal`] future.
    pub(crate) fn new(
        sql: SQL,
        exe: ExeFut,
        params: Vec<Encoded<'val>>,
        max_row: u32,
    ) -> Self {
        Self {
            sql,
            io: None,
            phase: Phase::Connect { f: exe },
            params,
            max_row,
        }
    }
}

impl<SQL, ExeFut, IO> Future for Portal<'_, SQL, ExeFut, IO>
where
    SQL: Sql + Unpin,
    ExeFut: Future<Output = Result<IO>> + Unpin,
    IO: PgTransport + Unpin,
{
    type Output = Result<IO>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();
        let sql = &mut me.sql;
        let io = &mut me.io;
        let phase = &mut me.phase;
        let params = &mut me.params;
        let max_row = me.max_row;

        loop {
            match &mut *phase {
                Phase::Connect { f } => {
                    let f = Pin::new(f);
                    let conn = ready!(f.poll(cx)?);
                    assert!(io.replace(conn).is_none());
                    *phase = Phase::Prepare;
                },
                Phase::Prepare => {
                    let data = ops::prepare(&*sql, params, io.as_mut().unwrap());
                    *phase = match data.cache_hit {
                        true => Phase::Portal(data),
                        false => Phase::PrepareFlush(data),
                    };
                },
                Phase::PrepareFlush(_) => {
                    ready!(io.as_mut().unwrap().poll_flush(cx)?);
                    let Phase::PrepareFlush(data) = mem::take(phase) else {
                        unreachable!()
                    };
                    *phase = Phase::PrepareComplete(data);
                }
                Phase::PrepareComplete(_) => {
                    let io = io.as_mut().unwrap();
                    ready!(io.poll_recv::<backend::ParseComplete>(cx)?);
                    let Phase::PrepareComplete(data) = mem::take(phase) else {
                        unreachable!()
                    };
                    io.add_stmt(data.sqlid, data.stmt.clone());
                    *phase = Phase::Portal(data);
                }
                Phase::Portal(data) => {
                    data.max_row = max_row;
                    ops::portal(data, params, &mut *io.as_mut().unwrap());
                    *phase = Phase::PortalFlush;
                }
                Phase::PortalFlush => {
                    ready!(io.as_mut().unwrap().poll_flush(cx)?);
                    *phase = Phase::Complete;
                    return Poll::Ready(Ok(io.take().unwrap()));
                }
                Phase::Invalid => unreachable!(),
                Phase::Complete => panic!("`poll` after complete"),
            }
        }
    }
}

