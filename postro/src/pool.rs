//! Database connection pooling.
use crate::{Connection, Result, executor::Executor, transport::PgTransport};

mod config;

#[cfg(feature = "tokio")]
mod worker;

pub use config::PoolConfig;

/// Database connection pool.
#[derive(Debug)]
pub struct Pool {
    conn: Option<Connection>,
    #[cfg(feature = "tokio")]
    handle: worker::WorkerHandle,
    #[cfg(not(feature = "tokio"))]
    handle: mock_handle::WorkerHandle,
}

impl Drop for Pool {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            self.handle.release(conn);
        }
    }
}

impl Clone for Pool {
    fn clone(&self) -> Self {
        Self {
            conn: None,
            handle: self.handle.clone(),
        }
    }
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
            Ok(Self { conn: None, handle })
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
            Self { conn: None, handle }
        }

        #[cfg(not(feature = "tokio"))]
        {
            let _ = config;
            panic!("runtime disabled")
        }
    }

    fn poll_connection(&mut self, cx: &mut std::task::Context) -> std::task::Poll<Result<Connection>> {
        self.handle.poll_acquire(cx)
    }
}

impl Executor for Pool {
    type Transport = PoolConnection<'static>;

    type Future = PoolConnect<'static>;

    fn connection(self) -> Self::Future {
        PoolConnect { pool: Some(PoolCow::Owned(self)) }
    }
}

impl Executor for &Pool {
    type Transport = PoolConnection<'static>;

    type Future = PoolConnect<'static>;

    fn connection(self) -> Self::Future {
        PoolConnect { pool: Some(PoolCow::Owned(self.clone())) }
    }
}

impl<'a> Executor for &'a mut Pool {
    type Transport = PoolConnection<'a>;

    type Future = PoolConnect<'a>;

    fn connection(self) -> Self::Future {
        PoolConnect { pool: Some(PoolCow::Borrow(self)) }
    }
}

/// Future returned from [`Pool`] implementation of [`Executor::connection`].
#[derive(Debug)]
pub struct PoolConnect<'a> {
    pool: Option<PoolCow<'a>>,
}

impl<'a> Future for PoolConnect<'a> {
    type Output = Result<PoolConnection<'a>>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        use std::task::Poll::*;
        if let Some(conn) = self.pool.as_mut().unwrap().as_mut().conn.take() {
            return Ready(Ok(PoolConnection { conn: Some(conn), pool: self.pool.take().unwrap() }))
        }
        let conn = std::task::ready!(self.pool.as_mut().unwrap().as_mut().poll_connection(cx)?);
        crate::common::verbose!(target: "pool_handle", "pool connection checkout");
        Ready(Ok(PoolConnection { conn: Some(conn), pool: self.pool.take().unwrap() }))
    }
}

/// Instance of [`Pool`] with the checked out connection.
#[derive(Debug)]
pub struct PoolConnection<'a> {
    pool: PoolCow<'a>,
    conn: Option<Connection>,
}

#[derive(Debug)]
enum PoolCow<'a> {
    Borrow(&'a mut Pool),
    Owned(Pool),
}

impl<'a> PoolCow<'a> {
    fn as_ref(&self) -> &Pool {
        match self {
            PoolCow::Borrow(pool) => *pool,
            PoolCow::Owned(pool) => &pool,
        }
    }

    fn as_mut(&mut self) -> &mut Pool {
        match self {
            PoolCow::Borrow(pool) => pool,
            PoolCow::Owned(pool) => pool,
        }
    }
}

impl PoolConnection<'_> {
    /// Returns the [`Pool`] handle.
    pub fn pool(&self) -> &Pool {
        self.pool.as_ref()
    }

    /// Returns the underlying [`Connection`].
    pub fn connection(&mut self) -> &mut Connection {
        // `conn` only `None` on drop
        self.conn.as_mut().unwrap()
    }
}

impl Drop for PoolConnection<'_> {
    fn drop(&mut self) {
        self.pool.as_mut().conn = self.conn.take();
    }
}

impl PgTransport for PoolConnection<'_> {
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

#[cfg(not(feature = "tokio"))]
mod mock_handle {
    use std::task::{Context, Poll};

    use crate::{Connection, Result};

    #[derive(Debug, Clone)]
    pub struct WorkerHandle;

    impl WorkerHandle {
        pub fn poll_acquire(&mut self, _: &mut Context) -> Poll<Result<Connection>> {
            unreachable!()
        }

        pub fn release(&self, _: Connection) {
            unreachable!()
        }
    }
}

