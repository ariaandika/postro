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
    Error, FromRow, Result, Row,
    encode::Encoded,
    ext::UsizeExt,
    postgres::{
        PgFormat,
        backend::{self, CommandComplete},
        frontend,
    },
    row::RowResult,
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
fn command_complete(cmd: backend::CommandComplete) -> u64 {
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

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct FetchStream<'val, SQL, ExeFut, IO, R> {
    sql: SQL,
    io: Option<IO>,
    data: Option<PrepareData>,
    phase: Phase<ExeFut>,
    params: Vec<Encoded<'val>>,
    max_row: u32,
    cmd: Option<CommandComplete>,
    _p: PhantomData<R>,
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

impl<'val, SQL, ExeFut, IO, R> FetchStream<'val, SQL, ExeFut, IO, R> {
    pub fn new(
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

impl<SQL, ExeFut, IO, R> Stream for FetchStream<'_, SQL, ExeFut, IO, R>
where
    SQL: Sql + Unpin,
    ExeFut: Future<Output = Result<IO>> + Unpin,
    IO: PgTransport + Unpin,
    R: FromRow + Unpin,
{
    type Item = Result<R>;

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
                            return Poll::Ready(Some(Err(err.into())));
                        },
                    }
                },
                Phase::DataRow(row) => {
                    use backend::BackendMessage::*;
                    match ready!(me.io.as_mut().unwrap().poll_recv(cx)?) {
                        DataRow(dr) => {
                            let row = row.inner_clone(dr.body);
                            let result = row.decode();
                            if result.is_err() {
                                me.io.as_mut().unwrap().ready_request();
                                me.phase = Phase::Complete;
                            }
                            return Poll::Ready(Some(result.map_err(Into::into)));
                        },

                        // `Execute` phase terminations:
                        CommandComplete(cmd) => {
                            me.cmd = Some(cmd);
                        },
                        PortalSuspended(_) => { },
                        EmptyQueryResponse(_) => {
                            me.phase = Phase::Complete;
                            return Poll::Ready(Some(Err(Error::empty_query())));
                        },
                        f => {
                            let err = f.unexpected("fetching data rows");
                            me.phase = Phase::Complete;
                            return Poll::Ready(Some(Err(err.into())));
                        },
                    }

                    me.phase = Phase::ReadyForQuery;
                },
                Phase::ReadyForQuery => {
                    ready!(me.io.as_mut().unwrap().poll_recv::<backend::ReadyForQuery>(cx)?);
                    me.phase = Phase::Complete;
                },
                Phase::Complete => return Poll::Ready(None),
            }
        }
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct FetchAll<'val, SQL, ExeFut, IO, R> {
    fetch: FetchStream<'val, SQL, ExeFut, IO, R>,
    output: Vec<R>,
}

impl<'val, SQL, ExeFut, IO, R> FetchAll<'val, SQL, ExeFut, IO, R> {
    pub fn new(sql: SQL, exe: ExeFut, params: Vec<Encoded<'val>>) -> Self {
        Self {
            fetch: FetchStream::new(sql, exe, params, 0),
            output: vec![],
        }
    }
}

impl<SQL, ExeFut, IO, R> Future for FetchAll<'_, SQL, ExeFut, IO, R>
where
    SQL: Sql + Unpin,
    ExeFut: Future<Output = Result<IO>> + Unpin,
    IO: PgTransport + Unpin,
    R: FromRow + Unpin,
{
    type Output = Result<Vec<R>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let me = self.get_mut();

        while let Some(r) = ready!(Pin::new(&mut me.fetch).poll_next(cx)?) {
            me.output.push(r);
        }

        Poll::Ready(Ok(std::mem::take(&mut me.output)))
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct FetchOne<'val, SQL, ExeFut, IO, R> {
    fetch: FetchStream<'val, SQL, ExeFut, IO, R>,
    output: Option<R>,
}

impl<'val, SQL, ExeFut, IO, R> FetchOne<'val, SQL, ExeFut, IO, R> {
    pub fn new(
        sql: SQL,
        exe: ExeFut,
        params: Vec<Encoded<'val>>,
    ) -> Self {
        Self {
            fetch: FetchStream::new(sql, exe, params, 1),
            output: None,
        }
    }
}

impl<SQL, ExeFut, IO, R> Future for FetchOne<'_, SQL, ExeFut, IO, R>
where
    SQL: Sql + Unpin,
    ExeFut: Future<Output = Result<IO>> + Unpin,
    IO: PgTransport + Unpin,
    R: FromRow + Unpin,
{
    type Output = Result<R>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let me = self.get_mut();

        while let Some(r) = ready!(Pin::new(&mut me.fetch).poll_next(cx)?) {
            me.output = Some(r);
        }

        match me.output.take() {
            Some(row) => Poll::Ready(Ok(row)),
            None => Ready(Err(Error::row_not_found())),
        }
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct FetchOptional<'val, SQL, ExeFut, IO, R> {
    fetch: FetchStream<'val, SQL, ExeFut, IO, R>,
    output: Option<R>,
}

impl<'val, SQL, ExeFut, IO, R> FetchOptional<'val, SQL, ExeFut, IO, R> {
    pub fn new(
        sql: SQL,
        exe: ExeFut,
        params: Vec<Encoded<'val>>,
    ) -> Self {
        Self {
            fetch: FetchStream::new(sql, exe, params, 1),
            output: None,
        }
    }
}

impl<SQL, ExeFut, IO, R> Future for FetchOptional<'_, SQL, ExeFut, IO, R>
where
    SQL: Sql + Unpin,
    ExeFut: Future<Output = Result<IO>> + Unpin,
    IO: PgTransport + Unpin,
    R: FromRow + Unpin,
{
    type Output = Result<Option<R>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let me = self.get_mut();

        while let Some(r) = ready!(Pin::new(&mut me.fetch).poll_next(cx)?) {
            me.output = Some(r);
        }

        match me.output.take() {
            Some(row) => Ready(Ok(Some(row))),
            None => Ready(Ok(None)),
        }
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Execute<'val, SQL, ExeFut, IO> {
    fetch: FetchStream<'val, SQL, ExeFut, IO, ()>,
}

impl<'val, SQL, ExeFut, IO> Execute<'val, SQL, ExeFut, IO> {
    pub fn new(
        sql: SQL,
        exe: ExeFut,
        params: Vec<Encoded<'val>>,
    ) -> Self {
        Self {
            fetch: FetchStream::new(sql, exe, params, 0),
        }
    }
}

impl<SQL, ExeFut, IO> Future for Execute<'_, SQL, ExeFut, IO>
where
    SQL: Sql + Unpin,
    ExeFut: Future<Output = Result<IO>> + Unpin,
    IO: PgTransport + Unpin,
{
    type Output = Result<RowResult>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let me = self.get_mut();

        while ready!(Pin::new(&mut me.fetch).poll_next(cx)?).is_some() { }

        match me.fetch.cmd.take() {
            Some(cmd) => Poll::Ready(Ok(RowResult { rows_affected: command_complete(cmd) })),
            None => todo!(),
        }
    }
}

