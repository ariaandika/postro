use std::{
    hash::{DefaultHasher, Hash, Hasher},
    mem,
    pin::Pin,
    task::{Context, Poll, ready},
};

use crate::{
    Result,
    column::ColumnInfo,
    encode::Encoded,
    ext::UsizeExt,
    postgres::{PgFormat, ProtocolError, backend, frontend},
    row::{FromRow, Row},
    statement::{PortalName, StatementName},
    transport::PgTransport,
};

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[project = FetchAllProject]
    pub struct FetchAll<'sql, 'val, R, IO> {
        sql: &'sql str,
        io: IO,
        phase: Phase,
        params: Vec<Encoded<'val>>,
        persistent: bool,
        output: Vec<R>,
    }
}

#[derive(Debug, Default)]
enum Phase {
    Prepare,
    PrepareFlush(PrepareData),
    PrepareComplete(PrepareData),
    Portal(PrepareData),
    PortalFlush/* (ExecuteData) */,
    PortalRecv {
        // data: ExecuteData,
        cols: Option<Vec<ColumnInfo>>,
    },
    #[default]
    Invalid,
}

#[derive(Debug)]
struct PrepareData {
    // sqlid: u64,
    stmt: StatementName,
}

// #[derive(Debug)]
// struct ExecuteData {
//     sqlid: u64,
//     stmt: StatementName,
//     portal: PortalName,
// }

// impl ExecuteData {
//     fn new(data: PrepareData, portal: PortalName) -> Self {
//         Self { sqlid: data.sqlid, stmt: data.stmt.clone(), portal }
//     }
// }

impl<'sql, 'val, R, IO> FetchAll<'sql, 'val, R, IO> {
    pub fn new(sql: &'sql str, io: IO, params: Vec<Encoded<'val>>, persistent: bool) -> Self {
        Self { sql, io, phase: Phase::Prepare, params, persistent, output: vec![] }
    }
}

impl<R, IO> Future for FetchAll<'_, '_, R, IO>
where
    R: FromRow,
    IO: PgTransport,
{
    type Output = Result<Vec<R>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let FetchAllProject {
            sql,
            io,
            phase,
            params,
            persistent,
            output,
        } = self.as_mut().project();

        loop {
            match &mut *phase {
                Phase::Prepare => {
                    let sqlid = {
                        let mut buf = DefaultHasher::new();
                        sql.hash(&mut buf);
                        buf.finish()
                    };

                    match !*persistent {
                        true => {
                            todo!()
                        }
                        false => match io.get_stmt(sqlid) {
                            Some(stmt) => {
                                *phase = Phase::Portal(PrepareData { /* sqlid, */ stmt });
                            },
                            None => {
                                let stmt = StatementName::next();
                                io.send(frontend::Parse {
                                    prepare_name: stmt.as_str(),
                                    sql: *sql,
                                    oids_len: params.len() as _,
                                    oids: params.iter().map(Encoded::oid),
                                });
                                io.send(frontend::Flush);
                                *phase = Phase::PrepareFlush(PrepareData { /* sqlid, */ stmt });
                            },
                        }
                    }
                },
                Phase::PrepareFlush(_) => {
                    ready!(io.poll_flush(cx)?);
                    let Phase::PrepareFlush(data) = mem::take(phase) else {
                        unreachable!()
                    };
                    *phase = Phase::PrepareComplete(data);
                },
                Phase::PrepareComplete(_) => {
                    ready!(io.poll_recv::<backend::ParseComplete>(cx)?);
                    let Phase::PrepareComplete(data) = mem::take(phase) else {
                        unreachable!()
                    };
                    *phase = Phase::Portal(data);
                },
                Phase::Portal(data) => {
                    let portal = PortalName::unnamed();

                    io.send(frontend::Bind {
                        portal_name: portal.as_str(),
                        stmt_name: data.stmt.as_str(),
                        param_formats_len: 1,
                        param_formats: [PgFormat::Binary],
                        params_len: params.len().to_u16(),
                        params_size_hint: params.iter().fold(0, |acc,n|{
                            acc + 4 + n.value().len().to_u32()
                        }),
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
                    let Phase::Portal(_) = mem::take(phase) else {
                        unreachable!()
                    };
                    *phase = Phase::PortalFlush/* (ExecuteData::new(data, portal)) */;
                },
                Phase::PortalFlush/* (_) */ => {
                    ready!(io.poll_flush(cx)?);
                    // let Phase::PortalFlush(data) = mem::take(phase) else {
                    //     unreachable!()
                    // };
                    *phase = Phase::PortalRecv { /* data, */ cols: None };
                },
                Phase::PortalRecv { /* data, */ cols } => {
                    use backend::BackendMessage::*;
                    loop {
                        match ready!(io.poll_recv(cx)?) {
                            BindComplete(_) => {},
                            RowDescription(rd) => {
                                cols.replace(ColumnInfo::decode_multi_vec(rd));
                            },
                            CommandComplete(_) => {},
                            ReadyForQuery(_) => break,
                            DataRow(dr) => {
                                let cols = cols.as_mut().expect("postgres didnt send RowDescription");
                                output.push(R::from_row(Row::new(cols, dr))?);
                            }
                            f => {
                                let err = ProtocolError::unexpected_phase(f.msgtype(), "extended query");
                                return Poll::Ready(Err(err.into()));
                            }
                        }
                    }
                    return Poll::Ready(Ok(mem::take(output)));
                }
                Phase::Invalid => unreachable!(),
            }
        }
    }
}

