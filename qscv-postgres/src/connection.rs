use std::num::NonZeroUsize;
use bytes::Bytes;
use lru::LruCache;

use crate::{
    common::general, encode::Encoded, error::{err, Result}, message::{
        authentication,
        frontend::{Bind, Execute, Parse, PasswordMessage, Query, Startup, Sync},
        BackendMessage,
    }, options::PgOptions, protocol::ProtocolError, raw_row::RawRow, stream::PgStream
};

const DEFAULT_PREPARED_STMT_CACHE: NonZeroUsize = NonZeroUsize::new(24).unwrap();

#[derive(Debug)]
pub struct PgConnection {
    stream: PgStream,
    #[allow(unused)]
    stmt_id: std::num::NonZeroU32,
    portal_id: std::num::NonZeroU32,
    prepared_stmt: LruCache<String, String>,
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
            let auth = match stream.recv().await? {
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
                    stream.send(PasswordMessage { password: opt.pass.as_ref() }).await?;
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

        Ok(Self {
            stream,
            stmt_id: std::num::NonZeroU32::new(1).unwrap(),
            portal_id: std::num::NonZeroU32::new(1).unwrap(),
            prepared_stmt: LruCache::new(DEFAULT_PREPARED_STMT_CACHE),
        })
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

    /// perform an extended query
    ///
    /// <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-EXT-QUERY>
    pub async fn query(&mut self, sql: &str, args: &[Encoded<'_>]) -> Result<Vec<RawRow>> {
        if let Some(_cached) = self.prepared_stmt.get_mut(sql) {
            todo!()
        }

        if self.stmt_id.checked_add(1).is_none() {
            self.stmt_id = std::num::NonZeroU32::new(1).unwrap();
        }

        if self.portal_id.checked_add(1).is_none() {
            self.portal_id = std::num::NonZeroU32::new(1).unwrap();
        }

        let mut b = itoa::Buffer::new();
        let prepare_name = b.format(self.stmt_id.get()).as_bytes();


        // In the extended protocol, the frontend first sends a Parse message

        self.stream.write(Parse {
            name: prepare_name,
            query: sql.as_bytes(),
            data_types_len: args.len() as _,
            data_types: args.into_iter().map(Encoded::oid),
        })?;

        // WARN: is this documented somewhere ?
        // Apparantly, sending Parse command, postgres does not immediately
        // response with ParseComplete.
        // 1. sending Sync will do so
        // 2. otherwise, we can continue the protocol without waiting for one
        //
        // self.stream.write(Sync)?;


        // Once a prepared statement exists, it can be readied for execution using a Bind message.

        let mut b2 = itoa::Buffer::new();
        let portal_name = b2.format(self.portal_id.get()).as_bytes();

        self.stream.write(Bind {
            portal_name,
            prepare_name,
            params_format_len: 1,
            params_format_code: [1],
            params_len: args.len() as _,
            params: args,
            results_format_len: 1,
            results_format_code: [1],
        })?;

        // Once a portal exists, it can be executed using an Execute message

        self.stream.write(Execute {
            portal_name,
            max_row: 0,
        })?;

        self.stream.write(Sync)?;
        self.stream.flush().await?;


        // The response to Parse is either ParseComplete or ErrorResponse
        dbg!(self.stream.recv::<BackendMessage>().await)?;

        // The response to Bind is either BindComplete or ErrorResponse.
        dbg!(self.stream.recv::<BackendMessage>().await)?;

        let mut rows = vec![];

        // The possible responses to Execute are the same as those described above
        // for queries issued via simple query protocol, except that Execute doesn't
        // cause ReadyForQuery or RowDescription to be issued.
        loop {
            match dbg!(self.stream.recv::<BackendMessage>().await)? {
                BackendMessage::DataRow(row) => {
                    rows.push(row.raw_row);
                },
                BackendMessage::CommandComplete(_) => break,
                _ => unreachable!(),
            }
        }

        // The response to Sync is either BindComplete or ErrorResponse.
        dbg!(self.stream.recv::<BackendMessage>().await)?;

        Ok(rows)
    }
}

#[cfg(feature = "tokio")]
#[test]
fn test_connect() {
    use crate::{encode::ValueRef, types::AsPgType};

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut conn = PgConnection::connect("postgres://cookiejar:cookie@127.0.0.1:5432/postgres").await.unwrap();
            let _ = conn
                .simple_query("select null,4").await.unwrap();

            let params = [
                Encoded::new(ValueRef::Bytes(b"DeezNutz".into()), str::PG_TYPE.oid()),
                Encoded::new(ValueRef::Bytes(b"FooBar".into()), str::PG_TYPE.oid()),
            ];

            let rows = conn
                .query(
                    "SELECT * FROM (VALUES\
                        ($1, null, $2),\
                        ($2, $1, null)\
                    ) AS t(column1, column2);",
                    &params
                )
                .await
                .unwrap();

            for row in rows {
                dbg!(row.collect::<Vec<_>>());
            }
        })
}

