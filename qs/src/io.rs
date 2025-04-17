use std::io;

use crate::{
    Result,
    message::{BackendProtocol, FrontendProtocol, frontend},
    statement::StatementName,
};

/// A buffered stream which can send and receive postgres message
pub trait PostgresIo {
    /// Future returned from [`flush`][PostgresIo::flush].
    type Flush<'a>: Future<Output = io::Result<()>> where Self: 'a;

    /// Future returned from [`recv`][PostgresIo::recv].
    type Recv<'a, B>: Future<Output = Result<B>> where B: BackendProtocol, Self: 'a;

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

    /// actually write buffered messages to underlying io
    fn flush(&mut self) -> Self::Flush<'_>;

    /// receive a backend message
    ///
    /// note that the implementor *should* detect database error,
    /// and return it as [`Result::Err`][std::result::Result::Err]
    fn recv<B: BackendProtocol>(&mut self) -> Self::Recv<'_, B>;

    /// Check for already prepared statement
    ///
    /// Only if the io support statement caching.
    fn get_stmt(&mut self, _sql: u64) -> Option<StatementName> {
        None
    }

    /// Add new prepared statement.
    ///
    /// Return `false` if caching is not supported,
    /// if so statement will be cleared immediately.
    fn add_stmt(&mut self, _sql: u64, _id: StatementName) -> bool {
        false
    }
}

impl<P> PostgresIo for &mut P where P: PostgresIo {
    type Flush<'a> = P::Flush<'a> where Self: 'a;

    type Recv<'a, B> = P::Recv<'a, B> where B: BackendProtocol, Self: 'a;

    fn send<F: FrontendProtocol>(&mut self, message: F) {
        P::send(self, message);
    }

    fn send_startup(&mut self, startup: frontend::Startup) {
        P::send_startup(self, startup);
    }

    fn flush(&mut self) -> Self::Flush<'_> {
        P::flush(self)
    }

    fn recv<B: BackendProtocol>(&mut self) -> Self::Recv<'_, B> {
        P::recv(self)
    }
}

