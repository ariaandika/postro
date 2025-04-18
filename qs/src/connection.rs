use lru::LruCache;
use std::num::NonZeroUsize;

use crate::{
    Result,
    transport::PgTransport,
    message::{BackendProtocol, FrontendProtocol, frontend},
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
        let mut stream = PgStream::connect(&opt).await?;

        let protocol::StartupResponse {
            backend_key_data: _,
            param_status: _,
        } = protocol::startup(&opt, &mut stream).await?;

        Ok(Self {
            stream,
            stmts: LruCache::new(DEFAULT_PREPARED_STMT_CACHE),
        })
    }
}

impl PgTransport for PgConnection {
    type Flush<'a> = <&'a mut PgStream as PgTransport>::Flush<'a> where Self: 'a;

    type Recv<'a, B> = <&'a mut PgStream as PgTransport>::Recv<'a, B> where B: BackendProtocol, Self: 'a;

    fn send<F: FrontendProtocol>(&mut self, message: F) {
        PgTransport::send(&mut self.stream, message);
    }

    fn send_startup(&mut self, startup: frontend::Startup) {
        PgTransport::send_startup(&mut self.stream, startup);
    }

    fn flush(&mut self) -> Self::Flush<'_> {
        PgTransport::flush(&mut self.stream)
    }

    fn recv<B: BackendProtocol>(&mut self) -> Self::Recv<'_, B> {
        PgTransport::recv(&mut self.stream)
    }

    fn get_stmt(&mut self, sqlid: u64) -> Option<StatementName> {
        self.stmts.get(&sqlid).cloned()
    }

    fn add_stmt(&mut self, sql: u64, id: StatementName) -> bool {
        self.stmts.push(sql, id);
        true
    }
}

