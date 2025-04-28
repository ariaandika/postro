use bytes::{Buf, BytesMut};
use lru::LruCache;
use std::{
    future::Ready,
    io,
    num::NonZeroUsize,
    task::{Context, Poll, ready},
};

use crate::{
    Result,
    executor::Executor,
    net::Socket,
    options::PgOptions,
    postgres::{
        BackendProtocol, ErrorResponse, FrontendProtocol, NoticeResponse, backend, frontend,
    },
    query::{self, StartupResponse},
    statement::StatementName,
    transport::{PgTransport, PgTransportExt},
};

const DEFAULT_BUF_CAPACITY: usize = 1024;
const DEFAULT_PREPARED_STMT_CACHE: NonZeroUsize = NonZeroUsize::new(24).unwrap();

/// Postgres Connection.
///
/// Connection cache a prepared statement transparently.
///
/// Connection handle `Sync` after receive an `ErrorResponse` message transparently.
///
/// Connection handle `NoticeResponse` message.
#[derive(Debug)]
pub struct PgConnection {
    // io
    socket: Socket,
    read_buf: BytesMut,
    write_buf: BytesMut,

    // feature
    stmts: LruCache<u64, StatementName>,

    // diagnostic
    sync_pending: usize,
}

impl PgConnection {
    /// perform a startup message via url
    pub async fn connect(url: &str) -> Result<Self> {
        Self::connect_with(PgOptions::parse(url)?).await
    }

    /// perform a startup message with options
    pub async fn connect_with(opt: PgOptions) -> Result<Self> {
        let socket = match &*opt.host {
            "localhost" => Socket::connect_socket(&(format!("/run/postgresql/.s.PGSQL.{}",opt.port))).await?,
            _ => Socket::connect_tcp(&opt.host, opt.port).await?,
        };

        let mut me = Self {
            socket,
            read_buf: BytesMut::with_capacity(DEFAULT_BUF_CAPACITY),
            write_buf: BytesMut::with_capacity(DEFAULT_BUF_CAPACITY),
            stmts: LruCache::new(DEFAULT_PREPARED_STMT_CACHE),
            sync_pending: 0,
        };

        let StartupResponse {
            backend_key_data: _,
            param_status: _,
        } = query::startup(&opt, &mut me).await?;

        Ok(me)
    }

    /// Gracefully close connection.
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
        #[cfg(feature = "log-verbose")]
        log::trace!("(B){:?}",backend::BackendMessage::decode($msgtype, $body.clone()).unwrap());
    };
}

impl PgConnection {
    pub fn healthcheck(&mut self) -> impl Future<Output = Result<()>> {
        std::future::poll_fn(|cx|self.poll_healthcheck(cx))
    }

    pub(crate) fn poll_healthcheck(&mut self, cx: &mut Context) -> Poll<Result<()>> {
        if !self.write_buf.is_empty() {
            ready!(self.poll_flush(cx)?)
        }

        while self.sync_pending != 0 {
            #[cfg(feature = "log-verbose")]
            log::trace!("healthcheck: {{sync_pending: {}}}",self.sync_pending);

            poll_message! {
                poll(self, cx);
                let msgtype;
                let body;
            }

            match msgtype {
                ErrorResponse::MSGTYPE => {
                    self.send(frontend::Sync);
                    // FIXME: the `Sync` will get eaten by ErrorResponse (need confirm)
                    self.ready_request();
                    #[cfg(feature = "log")]
                    log::error!("{}",ErrorResponse::new(body));
                },
                NoticeResponse::MSGTYPE => {
                    #[cfg(feature = "log")]
                    log::warn!("{}",NoticeResponse::new(body));
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

impl PgTransport for PgConnection {
    fn poll_flush(&mut self, cx: &mut Context) -> Poll<io::Result<()>> {
        crate::io::poll_write_all(&mut self.socket, &mut self.write_buf, cx)
    }

    fn poll_recv<B: BackendProtocol>(&mut self, cx: &mut Context) -> Poll<Result<B>> {
        ready!(self.poll_healthcheck(cx)?);

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
        #[cfg(feature = "log-verbose")]
        log::trace!("(F){message:?}");
        frontend::write(message, &mut self.write_buf);
    }

    fn send_startup(&mut self, startup: frontend::Startup) {
        #[cfg(feature = "log-verbose")]
        log::trace!("(F){startup:?}");
        startup.write(&mut self.write_buf);
    }

    fn get_stmt(&mut self, sqlid: u64) -> Option<StatementName> {
        self.stmts.get(&sqlid).cloned().inspect(|_e| {
            #[cfg(feature = "log-verbose")]
            log::trace!("prepare statement cache hit: {_e}")
        })
    }

    fn add_stmt(&mut self, sql: u64, id: StatementName) {
        #[cfg(feature = "log-verbose")]
        log::trace!("prepare statement add: {id}");
        if let Some((_id,name)) = self.stmts.push(sql, id) {
            #[cfg(feature = "log-verbose")]
            log::trace!("prepare statement removed: {name}");
            self.send(frontend::Close {
                variant: b'S',
                name: name.as_str(),
            });
            self.send(frontend::Sync);
            self.ready_request();
        }
    }
}

impl Executor for PgConnection {
    type Transport = Self;

    type Future = Ready<Self::Transport>;

    fn connection(self) -> Self::Future {
        std::future::ready(self)
    }
}
