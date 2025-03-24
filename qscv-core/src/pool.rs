//! asynchronous connection pool
//!
//! see [`Pool`] for details
use inner::PoolInner;
use std::{
    borrow::Cow,
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    connection::Connection,
    database::Database,
    transaction::Transaction,
    Error, Result,
};

mod options;
mod inner;
mod connection;
mod executor;
mod maybe;

pub use self::{
    options::{PoolOptions, PoolConnectionMetadata},
    connection::PoolConnection,
    maybe::MaybePoolConnection,
};

/// asynchronous connection pool
///
/// Constructor:
/// - [`Pool::connect`]
/// - [`Pool::connect_with`]
/// - [`Pool::connect_lazy`]
/// - [`Pool::connect_lazy_with`]
///
/// for available options see [`PoolOptions`]
///
/// core traits implementation:
/// - [`Acquire`][crate::acquire::Acquire], acquire a connection or start a transaction
/// - [`Executor`][crate::executor::Executor], execute a query
///
/// Transaction:
/// - [`Pool::begin`]
/// - [`Pool::try_begin`]
/// - [`Pool::begin_with`]
/// - [`Pool::try_begin_with`]
///
/// Destruction:
/// - [`Pool::close`]
///
/// # Sharing
///
/// `Pool` is `Send`, `Sync`, and `Clone`.
/// It is intended to be created once, and then shared across tasks.
///
/// # `Drop` behavior
///
/// Due to a lack of async `Drop`, dropping the last `Pool` handle may not immediately clean
/// up connections by itself. The connections will be dropped locally, which is sufficient for
/// SQLite, but for client/server databases like MySQL and Postgres, that only closes the
/// client side of the connection. The server will not know the connection is closed until
/// potentially much later: this is usually dictated by the TCP keepalive timeout in the server
/// settings.
///
/// Because the connection may not be cleaned up immediately on the server side, you may run
/// into errors regarding connection limits if you are creating and dropping many pools in short
/// order.
///
/// We recommend calling [`.close().await`] to gracefully close the pool and its connections
/// when you are done using it. This will also wake any tasks that are waiting on an `.acquire()`
/// call, so for long-lived applications it's a good idea to call `.close()` during shutdown.
pub struct Pool<DB: Database>(pub(crate) Arc<PoolInner<DB>>);

impl<DB: Database> Pool<DB> {
    /// Create a new connection pool with a default pool configuration and
    /// the given connection URL, and immediately establish one connection.
    ///
    /// The default configuration is mainly suited for testing and light-duty applications.
    /// For production applications, you'll likely want to make at least few tweaks.
    ///
    /// See [`PoolOptions::new()`] for details.
    pub async fn connect(url: &str) -> Result<Self> {
        PoolOptions::<DB>::new().connect(url).await
    }

    /// Create a new connection pool with a default pool configuration and
    /// the given `ConnectOptions`, and immediately establish one connection.
    ///
    /// The default configuration is mainly suited for testing and light-duty applications.
    /// For production applications, you'll likely want to make at least few tweaks.
    ///
    /// See [`PoolOptions::new()`] for details.
    pub async fn connect_with(
        options: <DB::Connection as Connection>::Options,
    ) -> Result<Self> {
        PoolOptions::<DB>::new().connect_with(options).await
    }

    /// Create a new connection pool with a default pool configuration and
    /// the given connection URL.
    ///
    /// The pool will establish connections only as needed.
    ///
    /// Refer to the relevant [`ConnectOptions`][crate::connection::ConnectOptions] impl for your database for the expected URL format:
    ///
    /// * Postgres: [`PgConnectOptions`][crate::postgres::PgConnectOptions]
    /// * MySQL: [`MySqlConnectOptions`][crate::mysql::MySqlConnectOptions]
    /// * SQLite: [`SqliteConnectOptions`][crate::sqlite::SqliteConnectOptions]
    /// * MSSQL: [`MssqlConnectOptions`][crate::mssql::MssqlConnectOptions]
    ///
    /// The default configuration is mainly suited for testing and light-duty applications.
    /// For production applications, you'll likely want to make at least few tweaks.
    ///
    /// See [`PoolOptions::new()`] for details.
    pub fn connect_lazy(url: &str) -> Result<Self> {
        PoolOptions::<DB>::new().connect_lazy(url)
    }

    /// Create a new connection pool with a default pool configuration and
    /// the given `ConnectOptions`.
    ///
    /// The pool will establish connections only as needed.
    ///
    /// The default configuration is mainly suited for testing and light-duty applications.
    /// For production applications, you'll likely want to make at least few tweaks.
    ///
    /// See [`PoolOptions::new()`] for details.
    pub fn connect_lazy_with(options: <DB::Connection as Connection>::Options) -> Self {
        PoolOptions::<DB>::new().connect_lazy_with(options)
    }

    /// Retrieves a connection from the pool.
    ///
    /// The total time this method is allowed to execute is capped by
    /// [`PoolOptions::acquire_timeout`].
    /// If that timeout elapses, this will return [`Error::PoolClosed`].
    ///
    /// ### Note: Cancellation/Timeout May Drop Connections
    /// If `acquire` is cancelled or times out after it acquires a connection from the idle queue or
    /// opens a new one, it will drop that connection because we don't want to assume it
    /// is safe to return to the pool, and testing it to see if it's safe to release could introduce
    /// subtle bugs if not implemented correctly. To avoid that entirely, we've decided to not
    /// gracefully handle cancellation here.
    ///
    /// However, if your workload is sensitive to dropped connections such as using an in-memory
    /// SQLite database with a pool size of 1, you can pretty easily ensure that a cancelled
    /// `acquire()` call will never drop connections by tweaking your [`PoolOptions`]:
    ///
    /// * Set [`test_before_acquire(false)`][PoolOptions::test_before_acquire]
    /// * Never set [`before_acquire`][PoolOptions::before_acquire] or
    ///   [`after_connect`][PoolOptions::after_connect].
    ///
    /// This should eliminate any potential `.await` points between acquiring a connection and
    /// returning it.
    pub fn acquire(&self) -> impl Future<Output = Result<PoolConnection<DB>>> + 'static {
        let shared = self.0.clone();
        async move { shared.acquire().await.map(|conn| conn.reattach()) }
    }

    /// Attempts to retrieve a connection from the pool if there is one available.
    ///
    /// Returns `None` immediately if there are no idle connections available in the pool
    /// or there are tasks waiting for a connection which have yet to wake.
    pub fn try_acquire(&self) -> Option<PoolConnection<DB>> {
        self.0.try_acquire().map(|conn| conn.into_live().reattach())
    }

    /// Retrieves a connection and immediately begins a new transaction.
    pub async fn begin(&self) -> Result<Transaction<'static, DB>, Error> {
        Transaction::begin(
            MaybePoolConnection::PoolConnection(self.acquire().await?),
            None,
        )
        .await
    }

    /// Attempts to retrieve a connection and immediately begins a new transaction if successful.
    pub async fn try_begin(&self) -> Result<Option<Transaction<'static, DB>>, Error> {
        match self.try_acquire() {
            Some(conn) => Transaction::begin(MaybePoolConnection::PoolConnection(conn), None)
                .await
                .map(Some),

            None => Ok(None),
        }
    }

    /// Retrieves a connection and immediately begins a new transaction using `statement`.
    pub async fn begin_with(
        &self,
        statement: impl Into<Cow<'static, str>>,
    ) -> Result<Transaction<'static, DB>, Error> {
        Transaction::begin(
            MaybePoolConnection::PoolConnection(self.acquire().await?),
            Some(statement.into()),
        )
        .await
    }

    /// Attempts to retrieve a connection and, if successful, immediately begins a new
    /// transaction using `statement`.
    pub async fn try_begin_with(
        &self,
        statement: impl Into<Cow<'static, str>>,
    ) -> Result<Option<Transaction<'static, DB>>> {
        match self.try_acquire() {
            Some(conn) => Transaction::begin(
                MaybePoolConnection::PoolConnection(conn),
                Some(statement.into()),
            )
            .await
            .map(Some),

            None => Ok(None),
        }
    }

    /// Shut down the connection pool, immediately waking all tasks waiting for a connection.
    ///
    /// Upon calling this method, any currently waiting or subsequent calls to [`Pool::acquire`] and
    /// the like will immediately return [`Error::PoolClosed`] and no new connections will be opened.
    /// Checked-out connections are unaffected, but will be gracefully closed on-drop
    /// rather than being returned to the pool.
    ///
    /// Returns a `Future` which can be `.await`ed to ensure all connections are
    /// gracefully closed. It will first close any idle connections currently waiting in the pool,
    /// then wait for all checked-out connections to be returned or closed.
    ///
    /// Waiting for connections to be gracefully closed is optional, but will allow the database
    /// server to clean up the resources sooner rather than later. This is especially important
    /// for tests that create a new pool every time, otherwise you may see errors about connection
    /// limits being exhausted even when running tests in a single thread.
    ///
    /// If the returned `Future` is not run to completion, any remaining connections will be dropped
    /// when the last handle for the given pool instance is dropped, which could happen in a task
    /// spawned by `Pool` internally and so may be unpredictable otherwise.
    ///
    /// `.close()` may be safely called and `.await`ed on multiple handles concurrently.
    pub fn close(&self) -> impl Future<Output = ()> + '_ {
        self.0.close()
    }

    /// Returns `true` if [`.close()`][Pool::close] has been called on the pool, `false` otherwise.
    pub fn is_closed(&self) -> bool {
        self.0.is_closed()
    }

    /// Returns the number of connections currently active. This includes idle connections.
    pub fn size(&self) -> u32 {
        self.0.size()
    }

    /// Returns the number of connections active and idle (not in use).
    pub fn num_idle(&self) -> usize {
        self.0.num_idle()
    }

    /// Gets a clone of the connection options for this pool
    pub fn connect_options(&self) -> Arc<<DB::Connection as Connection>::Options> {
        self.0
            .connect_options
            .read()
            .expect("write-lock holder panicked")
            .clone()
    }

    /// Updates the connection options this pool will use when opening any future connections.  Any
    /// existing open connection in the pool will be left as-is.
    pub fn set_connect_options(&self, connect_options: <DB::Connection as Connection>::Options) {
        // technically write() could also panic if the current thread already holds the lock,
        // but because this method can't be re-entered by the same thread that shouldn't be a problem
        let mut guard = self
            .0
            .connect_options
            .write()
            .expect("write-lock holder panicked");
        *guard = Arc::new(connect_options);
    }

    /// Get the options for this pool
    pub fn options(&self) -> &PoolOptions<DB> {
        &self.0.options
    }
}

impl<DB: Database> Clone for Pool<DB> {
    /// Returns a new [Pool] tied to the same shared connection pool.
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<DB: Database> fmt::Debug for Pool<DB> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Pool")
            .field("size", &self.0.size())
            .field("num_idle", &self.0.num_idle())
            .field("is_closed", &self.0.is_closed())
            .field("options", &self.0.options)
            .finish()
    }
}

/// get the time between the deadline and now and use that as our timeout
///
/// returns `Error::PoolTimedOut` if the deadline is in the past
fn deadline_as_timeout(deadline: Instant) -> Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .ok_or(Error::PoolTimedOut)
}

#[test]
#[allow(dead_code)]
fn assert_pool_traits() {
    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_clone<T: Clone>() {}

    fn assert_pool<DB: Database>() {
        assert_send_sync::<Pool<DB>>();
        assert_clone::<Pool<DB>>();
    }
}

