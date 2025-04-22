use lru::LruCache;
use std::num::NonZeroUsize;

use crate::{
    Result,
    transport::PgTransport,
    postgres::{BackendProtocol, FrontendProtocol, frontend},
    options::PgOptions,
    protocol,
    statement::StatementName,
    stream::PgStream,
};

const DEFAULT_PREPARED_STMT_CACHE: NonZeroUsize = NonZeroUsize::new(24).unwrap();

#[derive(Debug)]
pub struct PgConnection {
    stream: PgStream,
    stmts: LruCache<u64, StatementName>,
}

impl PgConnection {
    /// perform a startup message via url
    pub async fn connect(url: &str) -> Result<Self> {
        Self::connect_with(PgOptions::parse(url)?).await
    }

    /// perform a startup message with options
    pub async fn connect_with(opt: PgOptions) -> Result<Self> {
        let stream = PgStream::connect(&opt).await?;

        let mut me = Self {
            stream,
            stmts: LruCache::new(DEFAULT_PREPARED_STMT_CACHE),
        };

        let protocol::StartupResponse {
            backend_key_data: _,
            param_status: _,
        } = protocol::startup(&opt, &mut me).await?;

        Ok(me)
    }
}

impl PgTransport for PgConnection {
    fn poll_flush(&mut self, cx: &mut std::task::Context) -> std::task::Poll<std::io::Result<()>> {
        PgStream::poll_flush(&mut self.stream, cx)
    }

    fn poll_recv<B: BackendProtocol>(&mut self, cx: &mut std::task::Context) -> std::task::Poll<Result<B>> {
        PgStream::poll_recv(&mut self.stream, cx)
    }

    fn send<F: FrontendProtocol>(&mut self, message: F) {
        PgStream::send(&mut self.stream, message);
    }

    fn send_startup(&mut self, startup: frontend::Startup) {
        PgStream::send_startup(&mut self.stream, startup);
    }

    fn get_stmt(&mut self, sqlid: u64) -> Option<StatementName> {
        self.stmts.get(&sqlid).cloned()
    }

    fn add_stmt(&mut self, sql: u64, id: StatementName) {
        self.stmts.push(sql, id);
    }
}

