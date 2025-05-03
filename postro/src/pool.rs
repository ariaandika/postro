//! Database connection pooling.
use crate::{Connection, Result, executor::Executor, transport::PgTransport};

mod config;

#[cfg(feature = "tokio")]
mod worker;

pub use config::PoolConfig;

/// Database connection pool.
#[derive(Clone, Debug)]
pub struct Pool {
    #[cfg(feature = "tokio")]
    handle: worker::WorkerHandle,
}

impl Pool {
    /// Create [`Pool`] and try to create one connection.
    pub async fn connect(url: &str) -> Result<Self> {
        PoolConfig::from_env().connect(url).await
    }

    /// Create [`Pool`] without trying to create connection.
    pub fn connect_lazy(url: &str) -> Result<Self> {
        PoolConfig::from_env().connect_lazy(url)
    }

    /// Create [`Pool`] and try to create one connection.
    ///
    /// See [`Config::from_env`][1] for more details on env.
    ///
    /// [1]: crate::Config::from_env
    pub async fn connect_env() -> Result<Pool> {
        Self::connect_with(PoolConfig::from_env()).await
    }

    /// Create [`Pool`] and try to create one connection.
    pub async fn connect_with(config: PoolConfig) -> Result<Self> {
        #[cfg(feature = "tokio")]
        {
            let (handle,worker) = worker::WorkerHandle::new(config);
            tokio::spawn(worker);
            Ok(Self { handle })
        }

        #[cfg(not(feature = "tokio"))]
        {
            let _ = config;
            panic!("runtime disabled")
        }
    }

    /// Create [`Pool`] without trying to create connection.
    pub fn connect_lazy_with(config: PoolConfig) -> Self {
        #[cfg(feature = "tokio")]
        {
            let (handle,worker) = worker::WorkerHandle::new(config);
            tokio::spawn(worker);
            Self { handle }
        }

        #[cfg(not(feature = "tokio"))]
        {
            let _ = config;
            panic!("runtime disabled")
        }
    }

    fn poll_connection(&mut self, cx: &mut std::task::Context) -> std::task::Poll<Result<Connection>> {
        #[cfg(feature = "tokio")]
        {
            self.handle.poll_acquire(cx)
        }

        #[cfg(not(feature = "tokio"))]
        {
            let _ = cx;
            panic!("runtime disabled")
        }
    }
}

impl Executor for Pool {
    type Transport = PoolConnection;

    type Future = PoolConnect;

    fn connection(self) -> Self::Future {
        PoolConnect { pool: Some(self) }
    }
}

impl Executor for &Pool {
    type Transport = PoolConnection;

    type Future = PoolConnect;

    fn connection(self) -> Self::Future {
        PoolConnect { pool: Some(self.clone()) }
    }
}

impl Executor for &mut Pool {
    type Transport = PoolConnection;

    type Future = PoolConnect;

    fn connection(self) -> Self::Future {
        PoolConnect { pool: Some(self.clone()) }
    }
}

/// Future returned from [`Pool`] implementation of [`Executor::connection`].
#[derive(Debug)]
pub struct PoolConnect {
    pool: Option<Pool>,
}

impl Future for PoolConnect {
    type Output = Result<PoolConnection>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let conn = std::task::ready!(self.pool.as_mut().unwrap().poll_connection(cx)?);
        std::task::Poll::Ready(Ok(PoolConnection { conn: Some(conn), pool: self.pool.take().unwrap() }))
    }
}

/// Instance of [`Pool`] with the checked out connection.
#[derive(Debug)]
pub struct PoolConnection {
    pool: Pool,
    conn: Option<Connection>,
}

impl PoolConnection {
    /// Returns the [`Pool`] handle.
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    /// Returns the underlying [`Connection`].
    pub fn connection(&mut self) -> &mut Connection {
        // `conn` only `None` on drop
        self.conn.as_mut().unwrap()
    }
}

#[cfg(feature = "tokio")]
impl Drop for PoolConnection {
    fn drop(&mut self) {
        self.pool.handle.release(self.conn.take().unwrap());
    }
}

impl PgTransport for PoolConnection {
    fn poll_flush(&mut self, cx: &mut std::task::Context) -> std::task::Poll<std::io::Result<()>> {
        self.connection().poll_flush(cx)
    }

    fn poll_recv<B: crate::postgres::BackendProtocol>(&mut self, cx: &mut std::task::Context) -> std::task::Poll<Result<B>> {
        self.connection().poll_recv(cx)
    }

    fn ready_request(&mut self) {
        self.connection().ready_request();
    }

    fn send<F: crate::postgres::FrontendProtocol>(&mut self, message: F) {
        self.connection().send(message);
    }

    fn send_startup(&mut self, startup: crate::postgres::frontend::Startup) {
        self.connection().send_startup(startup);
    }

    fn get_stmt(&mut self, sql: u64) -> Option<crate::statement::StatementName> {
        self.connection().get_stmt(sql)
    }

    fn add_stmt(&mut self, sql: u64, id: crate::statement::StatementName) {
        self.connection().add_stmt(sql, id);
    }
}

