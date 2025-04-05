//! Postgres Protocol Operations
use crate::{
    common::general,
    io::PostgresIo,
    message::{backend, error::ProtocolError, frontend, BackendMessage},
    options::startup::StartupOptions,
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
            // TODO: NOTICE_RESPONSE
            f => Err(ProtocolError::unexpected_phase(f.msgtype(), "startup phase"))?,
        }
    }

    Ok(StartupResponse {
        param_status,
        backend_key_data: key_data.expect("postgres never send backend key data"),
    })
}

