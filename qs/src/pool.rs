use crate::{PgConnection, Result, executor::Executor, transport::PgTransport};

mod config;

#[cfg(feature = "tokio")]
mod worker;

pub use config::PoolConfig;

#[derive(Clone, Debug)]
pub struct Pool {
    #[cfg(feature = "tokio")]
    handle: worker::WorkerHandle,
}

impl Pool {
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

    pub fn connect_with(config: PoolConfig) -> Result<Self> {
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

    fn poll_connection(&mut self, cx: &mut std::task::Context) -> std::task::Poll<Result<PgConnection>> {
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

/// `PoolConnection` is a `Pool` which have guaranteed connection.
#[derive(Debug)]
pub struct PoolConnection {
    pool: Pool,
    conn: Option<PgConnection>,
}

impl PoolConnection {
    pub fn pool(&self) -> &Pool {
        &self.pool
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
        self.conn.as_mut().unwrap().poll_flush(cx)
    }

    fn poll_recv<B: crate::postgres::BackendProtocol>(&mut self, cx: &mut std::task::Context) -> std::task::Poll<Result<B>> {
        self.conn.as_mut().unwrap().poll_recv(cx)
    }

    fn ready_request(&mut self) {
        self.conn.as_mut().unwrap().ready_request();
    }

    fn send<F: crate::postgres::FrontendProtocol>(&mut self, message: F) {
        self.conn.as_mut().unwrap().send(message);
    }

    fn send_startup(&mut self, startup: crate::postgres::frontend::Startup) {
        self.conn.as_mut().unwrap().send_startup(startup);
    }

    fn get_stmt(&mut self, sql: u64) -> Option<crate::statement::StatementName> {
        self.conn.as_mut().unwrap().get_stmt(sql)
    }

    fn add_stmt(&mut self, sql: u64, id: crate::statement::StatementName) {
        self.conn.as_mut().unwrap().add_stmt(sql, id);
    }
}

