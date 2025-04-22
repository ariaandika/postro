use std::{io, task::{Context, Poll}};

use crate::{
    Result,
    postgres::{BackendProtocol, FrontendProtocol, frontend},
    statement::StatementName,
};

/// A buffered stream which can send and receive postgres message
pub trait PgTransport {
    fn poll_flush(&mut self, cx: &mut Context) -> Poll<io::Result<()>>;

    fn poll_recv<B: BackendProtocol>(&mut self, cx: &mut Context) -> Poll<Result<B>>;

    /// send message to the backend
    ///
    /// this does not actually write to the underlying io,
    /// instead implementor should buffer it
    ///
    /// use [`flush`][`PostgresIo::flush`] to actually send the message
    fn send<F: FrontendProtocol>(&mut self, message: F);

    /// send [`Startup`][1] message to the backend
    ///
    /// For historical reasons, the very first message sent by the client (the startup message)
    /// has no initial message-type byte.
    ///
    /// Thus, [`Startup`][1] does not implement [`FrontendProtocol`]
    ///
    /// [1]: frontend::Startup
    fn send_startup(&mut self, startup: frontend::Startup);

    /// Check for already prepared statement
    ///
    /// Only if the io support statement caching.
    fn get_stmt(&mut self, _sql: u64) -> Option<StatementName>;

    /// Add new prepared statement.
    ///
    /// Return `false` if caching is not supported,
    /// if so statement will be cleared immediately.
    fn add_stmt(&mut self, _sql: u64, _id: StatementName);
}

impl<P> PgTransport for &mut P where P: PgTransport {
    fn poll_flush(&mut self, cx: &mut Context) -> Poll<io::Result<()>> {
        P::poll_flush(self, cx)
    }

    fn poll_recv<B: BackendProtocol>(&mut self, cx: &mut Context) -> Poll<Result<B>> {
        P::poll_recv(self, cx)
    }

    fn send<F: FrontendProtocol>(&mut self, message: F) {
        P::send(self, message);
    }

    fn send_startup(&mut self, startup: frontend::Startup) {
        P::send_startup(self, startup);
    }

    fn get_stmt(&mut self, sql: u64) -> Option<StatementName> {
        P::get_stmt(self, sql)
    }

    fn add_stmt(&mut self, sql: u64, id: StatementName) {
        P::add_stmt(self, sql, id);
    }
}

pub trait PgTransportExt: PgTransport {
    fn flush(&mut self) -> impl Future<Output = io::Result<()>> {
        std::future::poll_fn(|cx|self.poll_flush(cx))
    }

    fn recv<B: BackendProtocol>(&mut self) -> impl Future<Output = Result<B>> {
        std::future::poll_fn(|cx|self.poll_recv(cx))
    }
}

impl<T> PgTransportExt for T where T: PgTransport { }

