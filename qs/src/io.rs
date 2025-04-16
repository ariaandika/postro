use std::io;

mod read_buf;

pub use read_buf::ReadBuf;

use crate::{
    message::{frontend::Startup, BackendProtocol, FrontendProtocol},
    stream::PgStream,
    Result,
};

/// A buffered stream which can send and receive postgres message
pub trait PostgresIo {
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
    fn flush(&mut self) -> impl Future<Output = io::Result<()>>;

    /// receive a backend message
    ///
    /// note that the implementor *should* detect database error,
    /// and return it as [`Result::Err`][std::result::Result::Err]
    fn recv<B: BackendProtocol>(&mut self) -> impl Future<Output = Result<B>>;
}

impl PostgresIo for &mut PgStream {
    fn send<F: FrontendProtocol>(&mut self, message: F) {
        PgStream::send(self, message);
    }

    fn send_startup(&mut self, startup: Startup) {
        PgStream::send_startup(self, startup);
    }

    fn flush(&mut self) -> impl Future<Output = io::Result<()>> {
        PgStream::flush(self)
    }

    fn recv<B: BackendProtocol>(&mut self) -> impl Future<Output = Result<B>> {
        PgStream::recv(self)
    }
}

impl<P> PostgresIo for &mut P where P: PostgresIo {
    fn send<F: FrontendProtocol>(&mut self, message: F) {
        P::send(self, message);
    }

    fn send_startup(&mut self, startup: Startup) {
        P::send_startup(self, startup);
    }

    fn flush(&mut self) -> impl Future<Output = io::Result<()>> {
        P::flush(self)
    }

    fn recv<B: BackendProtocol>(&mut self) -> impl Future<Output = Result<B>> {
        P::recv(self)
    }
}

