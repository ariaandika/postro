use bytes::Bytes;

use crate::{
    common::general,
    error::{err, Result},
    message::{
        authentication,
        frontend::{PasswordMessage, Query, Startup},
        BackendMessage,
    },
    options::PgOptions,
    protocol::ProtocolError,
    stream::PgStream,
};

#[derive(Debug)]
pub struct PgConnection {
    #[allow(unused)]
    stream: PgStream,
}

impl PgConnection {
    pub async fn connect(url: &str) -> Result<Self> {
        Self::connect_with(PgOptions::parse(url)?).await
    }

    pub async fn connect_with(opt: PgOptions) -> Result<Self> {
        let mut stream = PgStream::connect(&opt).await?;

        // https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-START-UP

        // To begin a session, a frontend opens a connection to the server and sends a startup message.

        stream.write(Startup {
            user: &opt.user,
            database: Some(&opt.dbname),
            replication: None,
        })?;

        stream.flush().await?;

        // The server then sends an appropriate authentication request message,
        // to which the frontend must reply with an appropriate authentication response message (such as a password).
        // For all authentication methods except GSSAPI, SSPI and SASL, there is at most one request and one response.
        // In some methods, no response at all is needed from the frontend, and so no authentication request occurs.
        // For GSSAPI, SSPI and SASL, multiple exchanges of packets may be needed to complete the authentication.

        loop {
            use authentication::Authentication::*;
            let auth = match stream.recv::<BackendMessage>().await? {
                BackendMessage::Authentication(ok) => ok,
                BackendMessage::ErrorResponse(err) => return Err(err.into()),
                f => return err!(Protocol,ProtocolError::new(general!(
                    "unexpected message in startup phase: ({f:?})",
                ))),
            };
            match auth {
                // we gucci
                Ok => break,
                // The frontend must now send a PasswordMessage containing the password in clear-text form
                CleartextPassword => {
                    let password = opt.pass.as_ref();
                    stream.write(PasswordMessage {
                        len: 4 + password.len() as i32,
                        password,
                    })?;
                    stream.flush().await?;
                },
                // TODO: support more authentication method
                f => return err!(Protocol,ProtocolError::new(general!(
                    "authentication {f:#?} is not yet supported",
                )))
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

        loop {
            let msg = stream.recv::<BackendMessage>().await?;
            match msg {
                BackendMessage::ReadyForQuery(_) => break,
                BackendMessage::BackendKeyData(_) => {}
                BackendMessage::ParameterStatus(_) => {}
                BackendMessage::ErrorResponse(err) => return Err(err.into()),
                f => return err!(Protocol,ProtocolError::new(general!(
                    "unexpected message in startup phase: {f:#?}",
                ))),
            }
        }

        Ok(Self { stream })
    }

    /// perform a simple query
    ///
    /// <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-SIMPLE-QUERY>
    pub async fn simple_query(&mut self, sql: impl Into<Bytes>) -> Result<()> {
        self.stream.write(Query::new(sql))?;
        self.stream.flush().await?;
        loop {
            match self.stream.recv::<BackendMessage>().await? {
                // Indicates that rows are about to be returned in response to a SELECT, FETCH, etc. query.
                // The contents of this message describe the column layout of the rows.
                // This will be followed by a DataRow message for each row being returned to the frontend.
                BackendMessage::RowDescription(_row) => { },
                // One of the set of rows returned by a SELECT, FETCH, etc. query.
                BackendMessage::DataRow(_columns) => { }
                // An SQL command completed normally
                BackendMessage::CommandComplete(_tag) => { }
                BackendMessage::ReadyForQuery(_) => break,
                f => return err!(Protocol,ProtocolError::new(general!(
                    "unexpected message in simple query: {f:#?}",
                ))),
            }
        }

        Ok(())
    }
}

#[cfg(feature = "tokio")]
#[test]
fn test_connect() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut conn = PgConnection::connect("postgres://cookiejar:cookie@127.0.0.1:5432/postgres").await.unwrap();
            let _ = conn.simple_query("select null,4").await.unwrap();
        })
}

