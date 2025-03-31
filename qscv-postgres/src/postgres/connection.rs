use super::{
    message::{authentication, startup::Startup, BackendMessage},
    options::PgOptions,
    stream::PgStream,
};

use crate::{common::general, error::{err, Result}, protocol::ProtocolError};

#[derive(Debug)]
pub struct PgConnection {
    #[allow(unused)]
    stream: PgStream,
}

impl PgConnection {
    pub async fn connect(url: &str) -> Result<Self> {
        let opt = PgOptions::parse(url)?;

        let mut stream = PgStream::connect(&opt).await?;

        // https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-START-UP

        // To begin a session, a frontend opens a connection to the server and sends a startup message.

        stream.write(Startup {
            user: &opt.user,
            database: None,
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
                Ok => break,
                // TODO: support more authentication method
                f => return err!(Protocol,ProtocolError::new(general!(
                    "authentication {f:?}) is not yet supported",
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
}

#[cfg(feature = "tokio")]
#[test]
fn test_connect() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _conn = PgConnection::connect("postgres://postgres:@localhost:5432/postgres").await.unwrap();
        })
}

