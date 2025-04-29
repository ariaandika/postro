use crate::{PgConnection, PgOptions, Result};

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

    async fn acquire(&mut self) -> PgConnection {
        #[cfg(feature = "tokio")]
        match self {
            PoolHandle::Worker(w) => w.acquire().await.unwrap(),
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

    pub async fn connection(&mut self) -> &mut PgConnection {
        if self.conn.is_none() {
            self.conn = Some(self.handle.acquire().await)
        }

        self.conn.as_mut().unwrap()
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            self.handle.release(conn);
        }
    }
}

