use bytes::{Buf, BytesMut};
use lru::LruCache;
use std::{
    io,
    num::NonZeroUsize,
    task::{Context, Poll, ready},
};

use crate::{
    Error, Result,
    net::Socket,
    options::PgOptions,
    postgres::{BackendProtocol, ErrorResponse, FrontendProtocol, NoticeResponse, frontend},
    query::{self, StartupResponse},
    statement::StatementName,
    transport::PgTransport,
};

const DEFAULT_BUF_CAPACITY: usize = 1024;
const DEFAULT_PREPARED_STMT_CACHE: NonZeroUsize = NonZeroUsize::new(24).unwrap();

#[derive(Debug)]
pub struct PgConnection {
    socket: Socket,
    read_buf: BytesMut,
    write_buf: BytesMut,

    // stream: PgStream,
    stmts: LruCache<u64, StatementName>,
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
        };

        let StartupResponse {
            backend_key_data: _,
            param_status: _,
        } = query::startup(&opt, &mut me).await?;

        Ok(me)
    }
}

impl PgTransport for PgConnection {
    fn poll_flush(&mut self, cx: &mut Context) -> Poll<io::Result<()>> {
        crate::io::poll_write_all(&mut self.socket, &mut self.write_buf, cx)
    }

    fn poll_recv<B: BackendProtocol>(&mut self, cx: &mut Context) -> Poll<Result<B>> {
        loop {
            let Some(mut header) = self.read_buf.get(..5) else {
                self.read_buf.reserve(1024);
                ready!(crate::io::poll_read(&mut self.socket, &mut self.read_buf, cx)?);
                continue;
            };

            let msgtype = header.get_u8();
            let len = header.get_i32() as _;

            if self.read_buf.len() - 1/*msgtype*/ < len {
                self.read_buf.reserve(1 + len);
                ready!(crate::io::poll_read(&mut self.socket, &mut self.read_buf, cx)?);
                continue;
            }

            self.read_buf.advance(5);
            let body = self.read_buf.split_to(len - 4).freeze();

            let res = match msgtype {
                ErrorResponse::MSGTYPE => {
                    let err = ErrorResponse::decode(msgtype, body).unwrap();
                    Err(Error::Database(err))?
                }
                NoticeResponse::MSGTYPE => {
                    todo!()
                }
                _ => B::decode(msgtype, body)?,
            };

            return Poll::Ready(Ok(res));
        }
    }

    fn send<F: FrontendProtocol>(&mut self, message: F) {
        frontend::write(message, &mut self.write_buf);
    }

    fn send_startup(&mut self, startup: frontend::Startup) {
        startup.write(&mut self.write_buf);
    }

    fn get_stmt(&mut self, sqlid: u64) -> Option<StatementName> {
        self.stmts.get(&sqlid).cloned()
    }

    fn add_stmt(&mut self, sql: u64, id: StatementName) {
        self.stmts.push(sql, id);
    }
}

