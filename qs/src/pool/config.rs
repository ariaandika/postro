use crate::{PgOptions, Result, common::ByteStr};

use super::Pool;

/// Pool configuration builder.
pub struct PoolConfig {
    pub(crate) conn: PgOptions,
    pub(crate) max_conn: usize,
}

impl PoolConfig {
    pub fn new() -> PoolConfig {
        Self {
            conn: PgOptions {
                user: ByteStr::default(),
                pass: ByteStr::default(),
                socket: None,
                host: ByteStr::default(),
                port: 0,
                dbname: ByteStr::default(),
            },
            max_conn: 10,
        }
    }

    /// Get connection config.
    pub fn connection(&self) -> &PgOptions {
        &self.conn
    }

    /// Set max connection value.
    pub fn max_connection(mut self, value: usize) -> Self {
        self.max_conn = value;
        self
    }
}

impl PoolConfig {
    pub fn connect(mut self, url: &str) -> Result<Pool> {
        let conn = PgOptions::parse(url)?;
        self.conn = conn;
        Pool::connect_with(self)
    }

    pub fn connect_lazy(mut self, url: &str) -> Result<Pool> {
        let conn = PgOptions::parse(url)?;
        self.conn = conn;
        Ok(Pool::connect_lazy_with(self))
    }
}

impl Pool {
    pub fn connect(url: &str) -> Result<Self> {
        PoolConfig::new().connect(url)
    }

    pub fn connect_lazy(url: &str) -> Result<Self> {
        PoolConfig::new().connect_lazy(url)
    }
}

