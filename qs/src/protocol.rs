//! Postgres Protocol Operations
use crate::{
    options::startup::StartupOptions, postgres::{backend, frontend, BackendMessage, ProtocolError}, row::FromRow, transport::PgTransport, Error, Result
};

/// Startup phase successful response.
pub struct StartupResponse {
    pub backend_key_data: backend::BackendKeyData,
    pub param_status: Vec<backend::ParameterStatus>,
}

/// Perform a startup message.
///
/// <https://www.postgresql.org/docs/17/protocol-flow.html#PROTOCOL-FLOW-START-UP>
pub async fn startup<'a, IO: PgTransport>(
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
        match io.recv().await? {
            // we gucci
            Ok => break,
            // The frontend must now send a PasswordMessage containing the password in clear-text form.
            CleartextPassword => {
                io.send(frontend::PasswordMessage { password: opt.get_password().unwrap_or_default() });
                io.flush().await?;
            },
            // TODO: support more authentication method
            _ => Err(Error::UnsupportedAuth)?
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
pub async fn simple_query<R: FromRow, IO: PgTransport>(sql: &str, mut io: IO) -> Result<Vec<R>> {
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
            DataRow(datarow) => todo!()/* rows.push(RowBuffer::new(datarow)) */,
            // An SQL command completed normally
            CommandComplete(_tag) => { }
            f => Err(ProtocolError::unexpected_phase(f.msgtype(), "simple query"))?,
        }
    }

    Ok(rows)
}

