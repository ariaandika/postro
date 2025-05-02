//! Postgres Connection
use bytes::{Buf, BytesMut};
use lru::LruCache;
use std::{
    future::Ready,
    io,
    num::NonZeroUsize,
    task::{Context, Poll, ready},
    time::Instant,
};

use crate::{
    Result,
    common::trace,
    executor::Executor,
    net::Socket,
    postgres::{
        BackendProtocol, ErrorResponse, FrontendProtocol, NoticeResponse, backend, frontend,
    },
    query::{self, StartupResponse},
    statement::StatementName,
    transport::{PgTransport, PgTransportExt},
};

mod config;

pub use config::{Config, ParseError};

const DEFAULT_BUF_CAPACITY: usize = 1024;
const DEFAULT_PREPARED_STMT_CACHE: NonZeroUsize = NonZeroUsize::new(24).unwrap();

/// Postgres Connection.
///
/// # Features
///
/// Connection cache a prepared statement. To opt out, use [`once`][1] when querying.
///
/// Connection handle `NoticeResponse` message. If the `log` feature is enabled,
/// `NoticeResponse` will be logged, otherwise it ignored.
///
/// Connection handle `Sync` after receive an `ErrorResponse` message.
/// This is postgres specific and happens transparently, most users
/// does not need to worry about this.
///
/// # Pending Messages
///
/// All RAII Guard API drop behavior are sync, so to perform async operation,
/// like sending rollback transaction, it can only be queued. Queued actions
/// is send on the next asynchronous operation. This is crucial for something
/// like failed transaction, where rollback can possibly delayed.
///
/// Note that with the [`Pool`][2] api, queued actions is executed automatically
/// when connection is released. The use of [`Connection`] directly
/// is only for short lived connection.
///
/// # Runtime
///
/// All constructor will panic if `tokio` features is not enabled.
///
/// [1]: crate::sql::SqlExt::once
/// [2]: crate::pool::Pool
#[derive(Debug)]
pub struct Connection {
    // io
    socket: Socket,
    read_buf: BytesMut,
    write_buf: BytesMut,

    // feature
    stmts: LruCache<u64, StatementName>,

    // diagnostic
    connected_at: Instant,
    sync_pending: usize,
}

impl Connection {
    /// Connect to postgres server via environment variables.
    ///
    /// See [`Config::from_env`] for more details.
    ///
    /// # Panics
    ///
    /// Panics if `tokio` feature is not enabled.
    pub fn connect_env() -> impl Future<Output = Result<Connection>> {
        Self::connect_with(Config::from_env())
    }

    /// Connect to postgres server via url.
    ///
    /// # Panics
    ///
    /// Panics if `tokio` feature is not enabled.
    pub async fn connect(url: &str) -> Result<Self> {
        Self::connect_with(Config::parse(url)?).await
    }

    /// Connect to postgres server with provided config.
    ///
    /// # Panics
    ///
    /// Panics if `tokio` feature is not enabled.
    pub async fn connect_with(config: Config) -> Result<Self> {
        let socket = if config.host == "localhost" {
            let socket = Socket::connect_socket(&(format!("/run/postgresql/.s.PGSQL.{}",config.port))).await;
            match socket {
                Ok(ok) => ok,
                Err(_) => Socket::connect_tcp(&config.host, config.port).await?,
            }
        } else {
            Socket::connect_tcp(&config.host, config.port).await?
        };

        let mut me = Self {
            socket,
            read_buf: BytesMut::with_capacity(DEFAULT_BUF_CAPACITY),
            write_buf: BytesMut::with_capacity(DEFAULT_BUF_CAPACITY),
            stmts: LruCache::new(DEFAULT_PREPARED_STMT_CACHE),
            connected_at: Instant::now(),
            sync_pending: 0,
        };

        let StartupResponse {
            backend_key_data: _,
            param_status: _,
        } = query::startup(&config, &mut me).await?;

        Ok(me)
    }
}

impl Connection {
    /// Get the [`Instant`] value of when the socket is connected to postgres server.
    pub fn connected_at(&self) -> Instant {
        self.connected_at
    }
}

impl Connection {
    /// Initiates or attempts to shut down socket, returning success when
    /// the I/O connection has completely shut down.
    pub fn poll_shutdown(&mut self, cx: &mut Context) -> Poll<io::Result<()>> {
        self.socket.poll_shutdown(cx)
    }

    /// Close connection cleanly.
    pub async fn close(mut self) -> io::Result<()> {
        self.send(frontend::Terminate);
        self.flush().await?;
        self.socket.shutdown().await
    }
}

macro_rules! poll_message {
    (
        poll($io:ident, $cx:ident);
        let $msgtype:ident;
        let $body:ident;
    ) => {
        let Some(mut header) = $io.read_buf.get(..5) else {
            $io.read_buf.reserve(1024);
            ready!(crate::io::poll_read(&mut $io.socket, &mut $io.read_buf, $cx)?);
            continue;
        };

        let $msgtype = header.get_u8();
        let len = header.get_i32() as _;

        if $io.read_buf.len() - 1/*msgtype*/ < len {
            $io.read_buf.reserve(1 + len);
            ready!(crate::io::poll_read(&mut $io.socket, &mut $io.read_buf, $cx)?);
            continue;
        }

        $io.read_buf.advance(5);
        let $body = $io.read_buf.split_to(len - 4).freeze();

        // Message fully acquired
        trace!("(B){:?}",backend::BackendMessage::decode($msgtype, $body.clone()).unwrap());
    };
}

impl Connection {
    /// Execute all queued action.
    ///
    /// See the struct module for [more details][1].
    ///
    /// [1]: Connection#pending-messages
    pub fn ready(&mut self) -> impl Future<Output = Result<()>> {
        std::future::poll_fn(|cx|self.poll_ready(cx))
    }

    /// Attempt to execute all queued action.
    ///
    /// See the struct module for [more details][1].
    ///
    /// [1]: Connection#pending-messages
    pub(crate) fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<()>> {
        if !self.write_buf.is_empty() {
            ready!(self.poll_flush(cx)?)
        }

        while self.sync_pending != 0 {
            trace!("healthcheck: {{sync_pending: {}}}",self.sync_pending);

            poll_message! {
                poll(self, cx);
                let msgtype;
                let _body;
            }

            match msgtype {
                ErrorResponse::MSGTYPE => {
                    self.send(frontend::Sync);
                    // NOTE:
                    // not documented but the `Sync` will get
                    // eaten by ErrorResponse based on currently happening
                    self.ready_request();
                    #[cfg(feature = "log")]
                    log::error!("{}",ErrorResponse::new(_body));
                },
                NoticeResponse::MSGTYPE => {
                    #[cfg(feature = "log")]
                    log::warn!("{}",NoticeResponse::new(_body));
                },
                backend::ReadyForQuery::MSGTYPE => {
                    self.sync_pending -= 1;
                },
                _ => {} // ignore all messages until `ReadyForQuery` received
            }
        }

        Poll::Ready(Ok(()))
    }
}

impl PgTransport for Connection {
    fn poll_flush(&mut self, cx: &mut Context) -> Poll<io::Result<()>> {
        crate::io::poll_write_all(&mut self.socket, &mut self.write_buf, cx)
    }

    fn poll_recv<B: BackendProtocol>(&mut self, cx: &mut Context) -> Poll<Result<B>> {
        ready!(self.poll_ready(cx)?);

        loop {
            poll_message! {
                poll(self, cx);
                let msgtype;
                let body;
            }

            match msgtype {
                ErrorResponse::MSGTYPE => {
                    self.send(frontend::Sync);
                    self.ready_request();
                    Err(ErrorResponse::new(body))?
                },
                NoticeResponse::MSGTYPE => {
                    #[cfg(feature = "log")]
                    log::warn!("{}",NoticeResponse::new(body));
                    continue;
                },
                _ => return Poll::Ready(Ok(B::decode(msgtype, body)?)),
            }
        }
    }

    fn ready_request(&mut self) {
        self.sync_pending += 1;
    }

    fn send<F: FrontendProtocol>(&mut self, message: F) {
        trace!("(F){message:?}");
        frontend::write(message, &mut self.write_buf);
    }

    fn send_startup(&mut self, startup: frontend::Startup) {
        trace!("(F){startup:?}");
        startup.write(&mut self.write_buf);
    }

    fn get_stmt(&mut self, sqlid: u64) -> Option<StatementName> {
        self.stmts.get(&sqlid).cloned().inspect(|_e|{
            trace!("statement cache hit: {_e}")
        })
    }

    fn add_stmt(&mut self, id: u64, name: StatementName) {
        trace!("statement added: {name}");

        if let Some((_id,name)) = self.stmts.push(id, name) {
            trace!("statement removed: {name}");

            self.send(frontend::Close {
                variant: b'S',
                name: name.as_str(),
            });
            self.send(frontend::Sync);
            self.ready_request();
        }
    }
}

impl Executor for Connection {
    type Transport = Self;

    type Future = Ready<Result<Self::Transport>>;

    fn connection(self) -> Self::Future {
        std::future::ready(Ok(self))
    }
}

