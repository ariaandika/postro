//! Postgres Protocol Operations
use crate::{
    common::general,
    io::PostgresIo,
    message::{backend, error::ProtocolError, frontend, BackendMessage},
    options::startup::StartupOptions,
    row_buffer::RowBuffer,
    Result,
};

/// startup phase successful response
pub struct StartupResponse {
    pub backend_key_data: backend::BackendKeyData,
    pub param_status: Vec<backend::ParameterStatus>,
}

/// perform a startup message
///
/// <https://www.postgresql.org/docs/17/protocol-flow.html#PROTOCOL-FLOW-START-UP>
pub async fn startup<'a, IO: PostgresIo>(
    opt: impl Into<StartupOptions<'a>>,
    mut io: IO,
) -> Result<StartupResponse> {
    let opt: StartupOptions = opt.into();

    // To begin a session, a frontend opens a connection to the server and sends a startup message.

    // (Optionally, the startup message can include additional settings for run-time parameters.)

    io.send_startup(frontend::Startup {
        user: opt.get_user(),
        database: opt.get_database(),
        replication: opt.get_replication(),
    });
    io.flush().await?;

    // The server then sends an appropriate authentication request message,
    // to which the frontend must reply with an appropriate authentication response message (such as a password).
    //
    // For all authentication methods except GSSAPI, SSPI and SASL, there is at most one request and one response.
    // In some methods, no response at all is needed from the frontend, and so no authentication request occurs.
    // For GSSAPI, SSPI and SASL, multiple exchanges of packets may be needed to complete the authentication.

    loop {
        use backend::Authentication::*;
        let auth = io.recv::<backend::Authentication>().await?;
        match auth {
            // we gucci
            Ok => break,
            // The frontend must now send a PasswordMessage containing the password in clear-text form
            CleartextPassword => {
                io.send(frontend::PasswordMessage { password: opt.get_password().unwrap_or_default() });
                io.flush().await?;
            },
            // TODO: support more authentication method
            _ => Err(ProtocolError::new(general!(
                "authentication {auth:?} is not yet supported",
            )))?
        }
    }

    // After having received AuthenticationOk, the frontend must wait for further messages from the server.
    // In this phase a backend process is being started, and the frontend is just an interested bystander.
    // It is still possible for the startup attempt to fail (ErrorResponse) or the server to decline support
    // for the requested minor protocol version (NegotiateProtocolVersion), but in the normal case the backend
    // will send some ParameterStatus messages, BackendKeyData, and finally ReadyForQuery.
    //
    // During this phase the backend will attempt to apply any additional run-time parameter settings that
    // were given in the startup message. If successful, these values become session defaults.
    // An error causes ErrorResponse and exit.

    let mut param_status = vec![];
    let mut key_data = None;

    loop {
        use BackendMessage::*;
        match io.recv().await? {
            ReadyForQuery(_) => break,
            BackendKeyData(new_key_data) => key_data = Some(new_key_data),
            ParameterStatus(param) => param_status.push(param),
            NoticeResponse(warn) => eprintln!("{warn}"),
            f => Err(ProtocolError::unexpected_phase(f.msgtype(), "startup phase"))?,
        }
    }

    Ok(StartupResponse {
        param_status,
        backend_key_data: key_data.expect("postgres never send backend key data"),
    })
}

/// perform a simple query
///
/// <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-SIMPLE-QUERY>
pub async fn simple_query<IO: PostgresIo>(sql: &str, mut io: IO) -> Result<Vec<RowBuffer>> {
    io.send(frontend::Query { sql });
    io.flush().await?;

    let mut rows = vec![];

    loop {
        use BackendMessage::*;
        match io.recv().await? {
            ReadyForQuery(_) => break,
            // Indicates that rows are about to be returned in response to a SELECT, FETCH, etc. query.
            // The contents of this message describe the column layout of the rows.
            // This will be followed by a DataRow message for each row being returned to the frontend.
            RowDescription(_row) => { },
            // One of the set of rows returned by a SELECT, FETCH, etc. query.
            DataRow(row) => rows.push(row.row_buffer),
            // An SQL command completed normally
            CommandComplete(_tag) => { }
            f => Err(ProtocolError::unexpected_phase(f.msgtype(), "simple query"))?,
        }
    }

    Ok(rows)
}

/// perform an extended query
///
/// this is simple flow of extended query protocol where prepared statement will not be cached,
/// and closed on query completion
///
/// <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-EXT-QUERY>
pub async fn extended_query<IO: PostgresIo>(
    sql: &str,
    args: &[crate::encode::Encoded<'_>],
    mut io: IO,
) -> Result<Vec<RowBuffer>> {
    const PREPARE_NAME: &str = "__xXtemp_prep_stmtXx__";
    const PORTAL_NAME: &str = "__xXtemp_nether_portalXx__";

    io.send(frontend::Parse {
        prepare_name: PREPARE_NAME,
        sql,
        data_types_len: args.len() as _,
        data_types: args.iter().map(crate::encode::Encoded::oid),
    });

    // Once a prepared statement exists, it can be readied for execution using a Bind message.

    io.send(frontend::Bind {
        portal_name: PORTAL_NAME,
        prepare_name: PREPARE_NAME,
        params_format_len: 1,
        params_format_code: [1],
        params_len: args,
        params: args,
        results_format_len: 1,
        results_format_code: [1],
    });

    // Once a portal exists, it can be executed using an Execute message

    io.send(frontend::Execute {
        portal_name: PORTAL_NAME,
        max_row: 0,
    });

    // A Flush must be sent after any extended-query command except Sync,
    // if the frontend wishes to examine the results of that command before issuing more commands.
    //
    // Without Flush, messages returned by the backend will be combined into the minimum possible
    // number of packets to minimize network overhead.
    io.send(frontend::Flush);

    io.flush().await?;

    // The response to Parse is either ParseComplete or ErrorResponse
    io.recv::<backend::ParseComplete>().await?;

    // The response to Bind is either BindComplete or ErrorResponse.
    io.recv::<backend::BindComplete>().await?;

    let mut rows = vec![];

    // The possible responses to Execute are the same as those described above
    // for queries issued via simple query protocol, except that Execute doesn't
    // cause ReadyForQuery or RowDescription to be issued.
    loop {
        use BackendMessage::*;
        match io.recv().await? {
            DataRow(row) => rows.push(row.row_buffer),
            CommandComplete(_) => break,
            f => Err(ProtocolError::unexpected_phase(f.msgtype(), "extended query"))?,
        }
    }

    // for this example, close the prepared statement immediately
    io.send(frontend::Close {
        variant: b'S',
        name: PREPARE_NAME,
    });
    io.send(frontend::Sync);
    io.flush().await?;

    // The response to Close is either CloseComplete or ErrorResponse.
    io.recv::<backend::CloseComplete>().await?;

    // The response to Sync is either BindComplete or ErrorResponse.
    io.recv::<backend::ReadyForQuery>().await?;

    Ok(rows)
}

#[cfg(all(test, feature = "tokio"))]
mod test {
    use crate::{encode::Encoded, stream::PgStream, PgOptions};

    #[test]
    fn test_connect() {
        use crate::{value::ValueRef, types::AsPgType};

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let opt = PgOptions::parse("postgres://cookiejar:cookie@127.0.0.1:5432/postgres").unwrap();
                let mut conn = PgStream::connect(
                    &opt,
                )
                .await
                .unwrap();

                super::startup(&opt, &mut conn).await.unwrap();

                let _ = super::simple_query("select null,4", &mut conn)
                    .await
                    .unwrap();

                let params = [
                    Encoded::new(ValueRef::Slice(&b"DeezNutz"[..]), str::PG_TYPE.oid()),
                    Encoded::new(ValueRef::Slice(&b"FooBar"[..]), str::PG_TYPE.oid()),
                ];

                let rows = super::extended_query(
                    "SELECT * FROM (VALUES\
                            ($1, null, $2),\
                            ($2, $1, null)\
                        ) AS t(column1, column2);",
                    &params,
                    &mut conn,
                )
                .await
                .unwrap();

                for row in rows {
                    dbg!(row.collect::<Vec<_>>());
                }
            })
    }
}


