use std::{
    hash::{DefaultHasher, Hash, Hasher},
    mem,
};

use crate::{
    encode::Encoded,
    ext::UsizeExt,
    postgres::{backend, frontend, PgFormat},
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

/// Write Prepare statement to `io`
///
/// If cache hit, no further action is required.
///
/// If cache miss, flushing is required, with responses possible:
/// - `ParseComplete` from `Parse`
///
/// Also caller might want to cache the returned statement.
pub fn prepare(
    sql: &str,
    params: &[Encoded],
    persistent: bool,
    mut io: impl PgTransport,
) -> PrepareData {
    let sqlid = {
        let mut buf = DefaultHasher::new();
        sql.hash(&mut buf);
        buf.finish()
    };

    if persistent {
        if let Some(stmt) = io.get_stmt(sqlid) {
            return PrepareData { sqlid, stmt, cache_hit: true, max_row: 0 };
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

    PrepareData { sqlid, stmt, cache_hit: false, max_row: 0 }
}

/// Write Prepare statement to `io`
///
/// Flushing is requied after call.
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
pub fn portal(data: &PrepareData, params: &mut Vec<Encoded>, mut io: impl PgTransport) {
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

/// Decode information from [`CommandComplete`][1] message
///
/// [1]: backend::CommandComplete
pub fn command_complete(cmd: backend::CommandComplete) -> u64 {
    let mut whs = cmd.tag.split_whitespace();
    let Some(tag) = whs.next() else {
        return 0;
    };

    if matches!(tag, "INSERT") {
        return whs
            .skip(1)
            .next()
            .and_then(|e| e.parse().ok())
            .unwrap_or_default();
    }

    match whs.next().unwrap() {
        "SELECT" => whs,
        "UPDATE" => whs,
        "DELETE" => whs,
        "MERGE" => whs,
        "FETCH" => whs,
        "MOVE" => whs,
        "COPY" => whs,
        _ => return 0,
    }

    .next()
    .and_then(|e| e.parse().ok())
    .unwrap_or_default()
}

