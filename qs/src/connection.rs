use lru::LruCache;
use std::num::NonZeroUsize;

#[allow(unused)]
use crate::{
    error::Result,
    io::PostgresIo,
    message::{BackendProtocol, FrontendProtocol, frontend},
    options::PgOptions,
    statement::StatementName,
    stream::PgStream,
};

const DEFAULT_PREPARED_STMT_CACHE: NonZeroUsize = NonZeroUsize::new(24).unwrap();

#[derive(Debug)]
#[allow(unused)]
pub struct PgConnection {
    stream: PgStream,
    stmt_id: std::num::NonZeroU32,
    portal_id: std::num::NonZeroU32,
    prepared_stmt: LruCache<String, String>,
    #[allow(unused)]
    prepared_stmt2: LruCache<u64, StatementName>,
}

impl PgConnection {
    /// perform a startup message via url
    pub async fn connect(url: &str) -> Result<Self> {
        Self::connect_with(PgOptions::parse(url)?).await
    }

    /// perform a startup message with options
    pub async fn connect_with(opt: PgOptions) -> Result<Self> {
        let mut stream = PgStream::connect(&opt).await?;

        let crate::protocol::StartupResponse {
            backend_key_data: _,
            param_status: _,
        } = crate::protocol::startup(&opt, &mut stream).await?;

        Ok(Self {
            stream,
            stmt_id: std::num::NonZeroU32::new(1).unwrap(),
            portal_id: std::num::NonZeroU32::new(1).unwrap(),
            prepared_stmt: LruCache::new(DEFAULT_PREPARED_STMT_CACHE),
            prepared_stmt2: LruCache::new(DEFAULT_PREPARED_STMT_CACHE),
        })
    }
}

impl PostgresIo for PgConnection {
    type Flush<'a> = <&'a mut PgStream as PostgresIo>::Flush<'a> where Self: 'a;

    type Recv<'a, B> = <&'a mut PgStream as PostgresIo>::Recv<'a, B> where B: BackendProtocol, Self: 'a;

    fn send<F: FrontendProtocol>(&mut self, message: F) {
        PostgresIo::send(&mut self.stream, message);
    }

    fn send_startup(&mut self, startup: frontend::Startup) {
        PostgresIo::send_startup(&mut &mut self.stream, startup);
    }

    fn flush<'a>(&'a mut self) -> Self::Flush<'a> {
        PostgresIo::flush(&mut self.stream)
    }

    fn recv<'a, B: BackendProtocol>(&'a mut self) -> Self::Recv<'a, B> {
        PostgresIo::recv(&mut self.stream)
    }
}

