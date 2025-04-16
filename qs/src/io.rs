use bytes::BytesMut;
use std::io;

use crate::{
    Result,
    message::{BackendProtocol, FrontendProtocol, frontend::Startup},
    net::WriteAllBuf,
    statement::StatementName,
    stream::{PgStream, Recv},
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

    /// send [`Startup`] message to the backend
    ///
    /// For historical reasons, the very first message sent by the client (the startup message)
    /// has no initial message-type byte.
    ///
    /// Thus, [`Startup`] does not implement [`FrontendProtocol`]
    fn send_startup(&mut self, startup: Startup);

    /// actually write buffered messages to underlying io
    fn flush<'a>(&'a mut self) -> Self::Flush<'a>;

    /// receive a backend message
    ///
    /// note that the implementor *should* detect database error,
    /// and return it as [`Result::Err`][std::result::Result::Err]
    fn recv<'a, B: BackendProtocol>(&'a mut self) -> Self::Recv<'a, B>;

    /// Check for already prepared statement
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

impl PostgresIo for &mut PgStream {
    type Flush<'a> = WriteAllBuf<'a, BytesMut> where Self: 'a;

    type Recv<'a, B> = Recv<'a, B> where B: BackendProtocol, Self: 'a;

    fn send<F: FrontendProtocol>(&mut self, message: F) {
        PgStream::send(self, message);
    }

    fn send_startup(&mut self, startup: Startup) {
        PgStream::send_startup(self, startup);
    }

    fn flush<'a>(&'a mut self) -> Self::Flush<'a> {
        PgStream::flush(self)
    }

    fn recv<'a, B: BackendProtocol>(&'a mut self) -> Self::Recv<'a, B> {
        PgStream::recv(self)
    }
}

impl<P> PostgresIo for &mut P where P: PostgresIo {
    type Flush<'a> = P::Flush<'a> where Self: 'a;

    type Recv<'a, B> = P::Recv<'a, B> where B: BackendProtocol, Self: 'a;

    fn send<F: FrontendProtocol>(&mut self, message: F) {
        P::send(self, message);
    }

    fn send_startup(&mut self, startup: Startup) {
        P::send_startup(self, startup);
    }

    fn flush<'a>(&'a mut self) -> Self::Flush<'a> {
        P::flush(self)
    }

    fn recv<'a, B: BackendProtocol>(&'a mut self) -> Self::Recv<'a, B> {
        P::recv(self)
    }
}

