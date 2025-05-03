use crate::{Config, Result};

use super::Pool;

/// Pool configuration builder.
pub struct PoolConfig {
    pub(crate) conn: Config,
    pub(crate) max_conn: usize,
}

impl PoolConfig {
    pub fn from_env() -> PoolConfig {
        Self {
            conn: Config::from_env(),
            max_conn: 10,
        }
    }

    /// Get connection config.
    pub fn connection(&self) -> &Config {
        &self.conn
    }

    /// Set max connection value.
    pub fn max_connection(mut self, value: usize) -> Self {
        self.max_conn = value;
        self
    }
}

impl PoolConfig {
    pub async fn connect(mut self, url: &str) -> Result<Pool> {
        let conn = Config::parse(url)?;
        self.conn = conn;
        Pool::connect_with(self).await
    }

    pub fn connect_lazy(mut self, url: &str) -> Result<Pool> {
        let conn = Config::parse(url)?;
        self.conn = conn;
        Ok(Pool::connect_lazy_with(self))
    }
}

