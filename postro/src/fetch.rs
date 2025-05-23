use futures_core::Stream;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    marker::PhantomData,
    mem,
    pin::Pin,
    task::{
        Context,
        Poll::{self, *},
        ready,
    },
};

use crate::{
    Result, Row,
    common::unit_error,
    encode::Encoded,
    ext::UsizeExt,
    postgres::{PgFormat, backend, frontend},
    sql::Sql,
    statement::{PortalName, StatementName},
    transport::PgTransport,
};

#[derive(Debug)]
pub struct PrepareData {
    pub sqlid: u64,
    pub stmt: StatementName,
    pub cache_hit: bool,
    /// this field intended to be edited by called for `portal` params.
    pub max_row: u32,
}

/// Write Prepare statement to `io`.
///
/// If cache hit, no further action is required.
///
/// If cache miss, flushing is required, with responses possible:
/// - `ParseComplete` from `Parse`
///
/// Also caller might want to cache the returned statement.
fn prepare(
    sql: &impl Sql,
    params: &[Encoded],
    mut io: impl PgTransport,
) -> PrepareData {
    let persist = sql.persistent();
    let sql = sql.sql().trim();

    let sqlid = {
        let mut buf = DefaultHasher::new();
        sql.hash(&mut buf);
        buf.finish()
    };

    if persist {
        if let Some(stmt) = io.get_stmt(sqlid) {
            return PrepareData { sqlid, stmt, cache_hit: true, max_row: 0 };
        }
    }

    let stmt = match persist {
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

    PrepareData { sqlid, stmt, cache_hit: false, max_row: 0 }
}

/// Write Prepare statement to `io`.
///
/// Flushing is required after call.
///
/// Responses possible:
/// - `BindComplete` from `Bind`
/// - `RowDescription` or `NoData` from `Describe`
/// - `DataRow` from `Execute`
/// - `Execute` phase is always terminated by the appearance of exactly one of these messages:
///   - `CommandComplete`
///   - `EmptyQueryResponse`
///   - `ErrorResponse`
///   - `PortalSuspended`
/// - `ReadyForQuery` from `Sync`
fn portal(data: &PrepareData, params: &mut Vec<Encoded>, mut io: impl PgTransport) {
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
        max_row: data.max_row,
    });
    io.send(frontend::Sync);
}

/// Decode information from [`CommandComplete`][1] message.
///
/// [1]: backend::CommandComplete
pub(crate) fn command_complete(cmd: backend::CommandComplete) -> u64 {
    let mut whs = cmd.tag.split_whitespace();
    let Some(tag) = whs.next() else {
        return 0;
    };
    let Some(rows) = whs.next() else {
        return 0;
    };
    match tag {
        "INSERT" => whs.next().unwrap_or_default(),
        "SELECT" => rows,
        "UPDATE" => rows,
        "DELETE" => rows,
        "MERGE" => rows,
        "FETCH" => rows,
        "MOVE" => rows,
        "COPY" => rows,
        _ => return 0,
    }
    .parse()
    .unwrap_or_default()
}

// ===== Fetch Stream and Future =====

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct FetchStream<'val, SQL, ExeFut, IO, M> {
    sql: SQL,
    io: Option<IO>,
    data: Option<PrepareData>,
    phase: Phase<ExeFut>,
    params: Vec<Encoded<'val>>,
    max_row: u32,
    cmd: Option<backend::CommandComplete>,
    _p: PhantomData<M>,
}

#[derive(Debug)]
enum Phase<ExeFut> {
    Connect { f: ExeFut },
    Prepare,
    PrepareComplete,
    Portal,
    BindComplete,
    Complete,
    RowDescription,
    DataRow(Row),
    ReadyForQuery,
}

impl<'val, SQL, ExeFut, IO, M> FetchStream<'val, SQL, ExeFut, IO, M> {
    pub(crate) fn new(
        sql: SQL,
        exe: ExeFut,
        params: Vec<Encoded<'val>>,
        max_row: u32,
    ) -> Self {
        Self {
            sql,
            io: None,
            data: None,
            phase: Phase::Connect { f: exe },
            params,
            max_row,
            cmd: None,
            _p: PhantomData,
        }
    }
}

impl<SQL, ExeFut, IO, M> Stream for FetchStream<'_, SQL, ExeFut, IO, M>
where
    SQL: Sql + Unpin,
    ExeFut: Future<Output = Result<IO>> + Unpin,
    IO: PgTransport + Unpin,
    M: StreamMap + Unpin,
{
    type Item = Result<M::Output>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let me = self.get_mut();

        loop {
            match &mut me.phase {
                Phase::Connect { f } => {
                    let io = ready!(Pin::new(f).poll(cx)?);
                    me.io = Some(io);
                    me.phase = Phase::Prepare;
                },
                Phase::Prepare => {
                    me.data = Some(prepare(&me.sql, &me.params, me.io.as_mut().unwrap()));
                    me.phase = match me.data.as_ref().unwrap().cache_hit {
                        true => Phase::Portal,
                        false => Phase::PrepareComplete,
                    };
                },
                Phase::PrepareComplete => {
                    let io = me.io.as_mut().unwrap();
                    let data = me.data.as_ref().unwrap();
                    ready!(io.poll_recv::<backend::ParseComplete>(cx)?);
                    io.add_stmt(data.sqlid, data.stmt.clone());
                    me.phase = Phase::Portal;
                },
                Phase::Portal => {
                    let data = me.data.as_mut().unwrap();
                    data.max_row = me.max_row;
                    portal(data, &mut me.params, me.io.as_mut().unwrap());
                    me.phase = Phase::BindComplete;
                },
                Phase::BindComplete => {
                    ready!(me.io.as_mut().unwrap().poll_recv::<backend::BindComplete>(cx)?);
                    me.phase = Phase::RowDescription;
                }
                Phase::RowDescription => {
                    use backend::BackendMessage::*;
                    match ready!(me.io.as_mut().unwrap().poll_recv(cx)?) {
                        NoData(_) => { },
                        // Received after `NoData`
                        CommandComplete(cmd) => {
                            me.cmd = Some(cmd);
                            me.phase = Phase::ReadyForQuery;
                        },

                        RowDescription(rd) => {
                            me.phase = Phase::DataRow(Row::new(rd.body));
                        },
                        f => {
                            let err = f.unexpected("description recv");
                            me.phase = Phase::Complete;
                            return Ready(Some(Err(err.into())));
                        },
                    }
                },
                Phase::DataRow(row) => {
                    use backend::BackendMessage::*;
                    match ready!(me.io.as_mut().unwrap().poll_recv(cx)?) {
                        DataRow(dr) => {
                            let row = row.inner_clone(dr.body);
                            let result = M::map(row);
                            if result.is_err() {
                                me.io.as_mut().unwrap().ready_request();
                                me.phase = Phase::Complete;
                            }
                            return Ready(Some(result));
                        },

                        // `Execute` phase terminations:
                        CommandComplete(cmd) => {
                            me.cmd = Some(cmd);
                        },
                        PortalSuspended(_) => { },
                        EmptyQueryResponse(_) => {
                            me.phase = Phase::Complete;
                            return Ready(Some(Err(EmptyQueryError.into())));
                        },
                        f => {
                            let err = f.unexpected("fetching data rows");
                            me.phase = Phase::Complete;
                            return Ready(Some(Err(err.into())));
                        },
                    }

                    me.phase = Phase::ReadyForQuery;
                },
                Phase::ReadyForQuery => {
                    ready!(me.io.as_mut().unwrap().poll_recv::<backend::ReadyForQuery>(cx)?);
                    me.phase = Phase::Complete;
                },
                Phase::Complete => return Ready(None),
            }
        }
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Fetch<'val, SQL, ExeFut, IO, M, C> {
    fetch: FetchStream<'val, SQL, ExeFut, IO, M>,
    collect: C,
}

impl<'val, SQL, ExeFut, IO, M, C> Fetch<'val, SQL, ExeFut, IO, M, C> {
    pub(crate) fn new(
        sql: SQL,
        exe: ExeFut,
        params: Vec<Encoded<'val>>,
        collect: C,
        max_row: u32,
    ) -> Self {
        Self {
            fetch: FetchStream::new(sql, exe, params, max_row),
            collect,
        }
    }
}

impl<SQL, ExeFut, IO, M, C> Future for Fetch<'_, SQL, ExeFut, IO, M, C>
where
    SQL: Sql + Unpin,
    ExeFut: Future<Output = Result<IO>> + Unpin,
    IO: PgTransport + Unpin,
    M: StreamMap + Unpin,
    C: FetchCollect<M::Output> + Unpin,
{
    type Output = Result<C::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let me = self.get_mut();

        while let Some(r) = ready!(Pin::new(&mut me.fetch).poll_next(cx)?) {
            me.collect.value(r);
        }

        Ready(me.collect.finish(me.fetch.cmd.take()))
    }
}

/// Adapter to process a [`Row`].
pub trait StreamMap {
    /// Processed row.
    type Output;

    /// Process row.
    fn map(row: Row) -> Result<Self::Output>;
}

/// Adapter to collect rows returned from [`FetchStream`] via [`Fetch`].
pub trait FetchCollect<Input> {
    /// Finished output item.
    type Output;

    /// Process found row.
    fn value(&mut self, input: Input);

    /// All rows collected, returns the result.
    fn finish(&mut self, cmd: Option<backend::CommandComplete>) -> Result<Self::Output>;
}

unit_error! {
    /// An error when try to query with empty string.
    pub struct EmptyQueryError("empty query string");
}

