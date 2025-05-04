use std::time::Duration;

use crate::{Config, Result};

use super::Pool;

/// Pool configuration builder.
pub struct PoolConfig {
    pub(crate) conn: Config,
    pub(crate) max_conn: usize,
    pub(crate) retry_delay: Duration,
    pub(crate) max_retry: usize,
    pub(crate) interval: Duration,
}

impl PoolConfig {
    pub fn from_env() -> PoolConfig {
        Self {
            conn: Config::from_env(),
            max_conn: 10,
            retry_delay: Duration::from_secs(5),
            max_retry: 3,
            interval: Duration::from_secs(60),
        }
    }

    /// Get connection config.
    pub fn connection(&self) -> &Config {
        &self.conn
    }

    /// Set max connection.
    pub fn max_connection(mut self, value: usize) -> Self {
        self.max_conn = value;
        self
    }

    /// Get retry delay.
    pub fn retry_delay(&self) -> Duration {
        self.retry_delay
    }

    /// Get max retry.
    pub fn max_retry(&self) -> usize {
        self.max_retry
    }

    /// Get interval.
    pub fn interval(&self) -> Duration {
        self.interval
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

