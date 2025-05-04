//! The [`PgTransport`] trait.
use std::{
    io,
    task::{Context, Poll},
};

use crate::{
    Result,
    postgres::{BackendProtocol, FrontendProtocol, frontend},
    statement::StatementName,
};

/// A buffered stream which can send and receive postgres message.
pub trait PgTransport: Unpin {
    /// Poll to flush the underlying io.
    fn poll_flush(&mut self, cx: &mut Context) -> Poll<io::Result<()>>;

    /// Poll to receive a message.
    ///
    /// Calling `poll_recv` will also try to [`poll_flush`][1] if there is buffered message.
    ///
    /// Implementor should handle `NoticeResponse` and should not return it.
    ///
    /// Implementor also should handle `ErrorResponse` and return it as [`Err`].
    ///
    /// [1]: PgTransport::poll_flush
    fn poll_recv<B: BackendProtocol>(&mut self, cx: &mut Context) -> Poll<Result<B>>;

    /// Request implementor to ignore all backend messages until `ReadyForQuery` is received.
    fn ready_request(&mut self);

    /// Send message to the backend.
    ///
    /// Note that this send is buffered, caller must also call
    /// [`poll_flush`][1] or [`flush`][2] afterwards.
    ///
    /// [1]: PgTransport::poll_flush
    /// [2]: PgTransportExt::flush
    fn send<F: FrontendProtocol>(&mut self, message: F);

    /// Send [`Startup`][1] message to the backend.
    ///
    /// For historical reasons, the very first message sent by the client (the startup message)
    /// has no initial message-type byte.
    ///
    /// Thus, [`Startup`][1] does not implement [`FrontendProtocol`]
    ///
    /// [1]: frontend::Startup
    fn send_startup(&mut self, startup: frontend::Startup);

    /// Check for already prepared statement.
    fn get_stmt(&mut self, sql: u64) -> Option<StatementName>;

    /// Add new prepared statement.
    fn add_stmt(&mut self, sql: u64, id: StatementName);
}

impl<P> PgTransport for &mut P where P: PgTransport {
    fn poll_flush(&mut self, cx: &mut Context) -> Poll<io::Result<()>> {
        P::poll_flush(self, cx)
    }

    fn poll_recv<B: BackendProtocol>(&mut self, cx: &mut Context) -> Poll<Result<B>> {
        P::poll_recv(self, cx)
    }

    fn ready_request(&mut self) {
        P::ready_request(self);
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

/// An extension trait to provide `Future` API for [`PgTransport`].
pub trait PgTransportExt: PgTransport {
    /// Flush the underlying io.
    fn flush(&mut self) -> impl Future<Output = io::Result<()>> {
        std::future::poll_fn(|cx|self.poll_flush(cx))
    }

    /// Receive a backend message.
    fn recv<B: BackendProtocol>(&mut self) -> impl Future<Output = Result<B>> {
        std::future::poll_fn(|cx|self.poll_recv(cx))
    }
}

impl<T> PgTransportExt for T where T: PgTransport { }

