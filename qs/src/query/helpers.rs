use std::borrow::Cow;

use crate::{
    Error, Result,
    error::ErrorKind,
    postgres::{
        BackendMessage,
        backend::{self, RowDescription},
        frontend,
    },
    row::{FromRow, Row},
    transport::{PgTransport, PgTransportExt},
};

/// Config for postgres startup phase.
///
/// <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-START-UP>
pub struct StartupConfig<'a> {
    pub(crate) user: Cow<'a,str>,
    pub(crate) database: Option<Cow<'a,str>>,
    pub(crate) password: Option<Cow<'a,str>>,
    pub(crate) replication: Option<Cow<'a,str>>,
}

/// Startup phase successful response.
pub struct StartupResponse {
    /// This message provides secret-key data that the frontend must
    /// save if it wants to be able to issue cancel requests later.
    pub backend_key_data: backend::BackendKeyData,
    /// This message informs the frontend about the current (initial) setting
    /// of backend parameters, such as client_encoding or date style.
    pub param_status: Vec<backend::ParameterStatus>,
}

/// Perform a startup message.
///
/// <https://www.postgresql.org/docs/17/protocol-flow.html#PROTOCOL-FLOW-START-UP>
pub async fn startup<'a, IO: PgTransport>(
    opt: impl Into<StartupConfig<'a>>,
    mut io: IO,
) -> Result<StartupResponse> {

    let opt: StartupConfig = opt.into();

    // To begin a session, a frontend opens a connection to the server and sends a startup message.

    // (Optionally, the startup message can include additional settings for run-time parameters.)

    io.send_startup(frontend::Startup {
        user: opt.user(),
        database: opt.database(),
        replication: opt.replication(),
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
                io.send(frontend::PasswordMessage { password: opt.password().unwrap_or_default() });
                io.flush().await?;
            },
            // TODO: support more authentication method
            _ => Err(Error::from(ErrorKind::UnsupportedAuth))?
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
            f => Err(f.unexpected("startup phase"))?,
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

    // Indicates that rows are about to be returned in response to a SELECT, FETCH, etc. query.
    // The contents of this message describe the column layout of the rows.
    // This will be followed by a DataRow message for each row being returned to the frontend.
    let row_data = Row::new(io.recv::<RowDescription>().await?.body);

    let mut rows = vec![];

    loop {
        use BackendMessage::*;
        match io.recv().await? {
            ReadyForQuery(_) => break,
            // One of the set of rows returned by a SELECT, FETCH, etc. query.
            DataRow(dr) => rows.push(R::from_row(row_data.inner_clone(dr.body))?),
            // An SQL command completed normally
            CommandComplete(_tag) => { }
            f => Err(f.unexpected("simple query"))?,
        }
    }

    Ok(rows)
}

impl<'a> StartupConfig<'a> {
    /// Create new config, the database user name is required.
    pub fn new(user: impl Into<Cow<'a, str>>) -> Self {
        Self { user: user.into(), database: None, password: None, replication: None  }
    }

    /// The database user name to connect as.
    pub fn user(&self) -> &str {
        &self.user
    }

    /// The database to connect to. Defaults to the user name.
    pub fn database(&self) -> Option<&str> {
        self.database.as_ref().map(<_>::as_ref)
    }

    /// The database to connect to. Defaults to the user name.
    pub fn set_database(&mut self, database: impl Into<Cow<'a,str>>) {
        self.database = Some(database.into());
    }

    /// Authentication password, the default is empty string.
    pub fn password(&self) -> Option<&str> {
        self.password.as_ref().map(<_>::as_ref)
    }

    /// Authentication password, the default is empty string.
    pub fn set_password(&mut self, password: impl Into<Cow<'a,str>>) {
        self.password = Some(password.into());
    }

    /// Used to connect in streaming replication mode, where a small set of replication commands can be issued
    /// instead of SQL statements.
    ///
    /// Value can be true, false, or database, and the default is false.
    ///
    /// See [Section 53.4](https://www.postgresql.org/docs/current/protocol-replication.html) for details.
    pub fn replication(&self) -> Option<&str> {
        self.replication.as_ref().map(<_>::as_ref)
    }

    /// Used to connect in streaming replication mode, where a small set of replication commands can be issued
    /// instead of SQL statements.
    ///
    /// Value can be true, false, or database, and the default is false.
    ///
    /// See [Section 53.4](https://www.postgresql.org/docs/current/protocol-replication.html) for details.
    pub fn set_replication(&mut self, replication: impl Into<Cow<'a,str>>) {
        self.replication = Some(replication.into());
    }
}
