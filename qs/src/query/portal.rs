use std::{
    hash::{DefaultHasher, Hash, Hasher},
    marker::PhantomData,
    mem,
    pin::Pin,
    task::{Context, Poll, ready},
};

use crate::{
    Result,
    encode::Encoded,
    ext::UsizeExt,
    postgres::{PgFormat, backend, frontend},
    row::FromRow,
    statement::{PortalName, StatementName},
    transport::PgTransport,
};

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[project = PortalProject]
    pub struct Portal<'sql, 'val, R, IO> {
        sql: &'sql str,
        io: Option<IO>,
        phase: Phase,
        params: Vec<Encoded<'val>>,
        persistent: bool,
        _p: PhantomData<R>,
    }
}

impl<'sql, 'val, R, IO> Portal<'sql, 'val, R, IO> {
    pub fn new(sql: &'sql str, io: IO, params: Vec<Encoded<'val>>, persistent: bool) -> Self {
        Self {
            sql,
            io: Some(io),
            phase: Phase::Prepare,
            params,
            persistent,
            _p: PhantomData,
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

#[derive(Debug)]
struct PrepareData {
    sqlid: u64,
    stmt: StatementName,
}

impl<R, IO> Future for Portal<'_, '_, R, IO>
where
    R: FromRow,
    IO: PgTransport,
{
    type Output = Result<IO>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let PortalProject {
            sql,
            io: self_io,
            phase,
            params,
            persistent,
            _p,
        } = self.as_mut().project();

        let io = self_io.as_mut().expect("foo poll after complete");

        loop {
            match &mut *phase {
                Phase::Prepare => {
                    let sqlid = {
                        let mut buf = DefaultHasher::new();
                        sql.hash(&mut buf);
                        buf.finish()
                    };

                    if *persistent {
                        if let Some(stmt) = io.get_stmt(sqlid) {
                            *phase = Phase::Portal(PrepareData { sqlid, stmt });
                            continue;
                        }
                    }

                    let stmt = match persistent {
                        true => StatementName::next(),
                        false => StatementName::unnamed(),
                    };

                    io.send(frontend::Parse {
                        prepare_name: stmt.as_str(),
                        sql,
                        oids_len: params.len() as _,
                        oids: params.iter().map(Encoded::oid),
                    });
                    io.send(frontend::Flush);

                    *phase = Phase::PrepareFlush(PrepareData { sqlid, stmt });
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
                    let portal = PortalName::unnamed();

                    io.send(frontend::Bind {
                        portal_name: portal.as_str(),
                        stmt_name: data.stmt.as_str(),
                        param_formats_len: 1,
                        param_formats: [PgFormat::Binary],
                        params_len: params.len().to_u16(),
                        params_size_hint: params
                            .iter()
                            .fold(0, |acc, n| acc + 4 + n.value().len().to_u32()),
                        params: mem::take(params).into_iter(),
                        result_formats_len: 1,
                        result_formats: [PgFormat::Binary],
                    });
                    io.send(frontend::Describe {
                        kind: b'P',
                        name: portal.as_str(),
                    });
                    io.send(frontend::Execute {
                        portal_name: portal.as_str(),
                        max_row: 0,
                    });
                    io.send(frontend::Sync);

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

