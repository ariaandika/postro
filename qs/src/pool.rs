use crate::{executor::Executor, transport::PgTransport, PgConnection, PgOptions, Result};

#[cfg(feature = "tokio")]
mod worker;

#[derive(Clone, Debug)]
enum PoolHandle {
    #[cfg(feature = "tokio")]
    Worker(worker::WorkerHandle),
}

impl PoolHandle {
    fn new_worker(_config: PoolConfig) -> PoolHandle {
        #[cfg(feature = "tokio")]
        {
            let (handle,worker) = worker::WorkerHandle::new(_config);
            tokio::spawn(worker);
            Self::Worker(handle)
        }

        #[cfg(not(feature = "tokio"))]
        {
            panic!("runtime disabled")
        }
    }

    fn poll_acquire(&mut self, cx: &mut std::task::Context) -> std::task::Poll<Result<PgConnection>> {
        #[cfg(feature = "tokio")]
        match self {
            PoolHandle::Worker(w) => w.poll_acquire(cx),
        }

        #[cfg(not(feature = "tokio"))]
        {
            panic!("runtime disabled")
        }
    }

    fn release(&mut self, _conn: PgConnection) {
        #[cfg(feature = "tokio")]
        match self {
            PoolHandle::Worker(w) => w.release(_conn),
        }
    }
}

#[derive(Debug)]
pub struct PoolConfig {
    conn: PgOptions,
    max_conn: usize,
}

impl PoolConfig {
    pub fn max_connection(&self) -> usize {
        self.max_conn
    }
}

#[derive(Debug)]
pub struct Pool {
    conn: Option<PgConnection>,
    handle: PoolHandle,
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
    pub fn connect_lazy(url: &str) -> Result<Self> {
        Ok(Self {
            conn: None,
            handle: PoolHandle::new_worker(PoolConfig {
                conn: PgOptions::parse(url)?,
                max_conn: 10,
            }),
        })
    }

    pub fn with(config: PoolConfig) -> Self {
        Self {
            conn: None,
            handle: PoolHandle::new_worker(config),
        }
    }

    fn poll_connection<'a>(&'a mut self, cx: &mut std::task::Context) -> std::task::Poll<Result<&'a mut PgConnection>> {
        match self.conn.is_some() {
            true => std::task::Poll::Ready(Ok(self.conn.as_mut().expect("for fuck sake"))),
            false => {
                let acq = std::task::ready!(self.handle.poll_acquire(cx)?);
                assert!(self.conn.replace(acq).is_none());
                std::task::Poll::Ready(Ok(self.conn.as_mut().unwrap()))
            }
        }
    }

    // pub async fn connection(&mut self) -> &mut PgConnection {
    //     if self.conn.is_none() {
    //         self.conn = Some(self.handle.acquire().await)
    //     }
    //
    //     self.conn.as_mut().unwrap()
    // }
}

impl Drop for Pool {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            self.handle.release(conn);
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

pub struct PoolConnect {
    pool: Option<Pool>,
}

impl Future for PoolConnect {
    type Output = Result<PoolConnection>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        std::task::ready!(self.pool.as_mut().unwrap().poll_connection(cx)?);
        std::task::Poll::Ready(Ok(PoolConnection { pool: self.pool.take().unwrap() }))
    }
}

/// `PoolConnection` is a `Pool` which have guaranteed connection.
pub struct PoolConnection {
    pool: Pool,
}

impl PgTransport for PoolConnection {
    fn poll_flush(&mut self, cx: &mut std::task::Context) -> std::task::Poll<std::io::Result<()>> {
        self.pool.conn.as_mut().unwrap().poll_flush(cx)
    }

    fn poll_recv<B: crate::postgres::BackendProtocol>(&mut self, cx: &mut std::task::Context) -> std::task::Poll<Result<B>> {
        self.pool.conn.as_mut().unwrap().poll_recv(cx)
    }

    fn ready_request(&mut self) {
        self.pool.conn.as_mut().unwrap().ready_request();
    }

    fn send<F: crate::postgres::FrontendProtocol>(&mut self, message: F) {
        self.pool.conn.as_mut().unwrap().send(message);
    }

    fn send_startup(&mut self, startup: crate::postgres::frontend::Startup) {
        self.pool.conn.as_mut().unwrap().send_startup(startup);
    }

    fn get_stmt(&mut self, sql: u64) -> Option<crate::statement::StatementName> {
        self.pool.conn.as_mut().unwrap().get_stmt(sql)
    }

    fn add_stmt(&mut self, sql: u64, id: crate::statement::StatementName) {
        self.pool.conn.as_mut().unwrap().add_stmt(sql, id);
    }
}

