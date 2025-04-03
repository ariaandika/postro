use std::io;

use crate::{
    message::{frontend, FrontendMessage},
    net::{BufferedSocket, Socket},
    protocol::{ProtocolDecode, ProtocolEncode, ProtocolError},
    PgOptions, Result,
};

#[derive(Debug)]
pub struct PgStream {
    socket: BufferedSocket,
}

impl PgStream {
    pub async fn connect(opt: &PgOptions) -> Result<Self> {
        let socket = match &*opt.host {
            "localhost" => Socket::connect_socket(&(format!("/run/postgresql/.s.PGSQL.{}",opt.port))).await?,
            _ => Socket::connect_tcp(&opt.host, opt.port).await?,
        };

        Ok(Self { socket: BufferedSocket::new(socket) })
    }

    /// write message to a buffer, this does not write to underlying io
    pub fn write<E>(&mut self, message: E) -> Result<(), ProtocolError>
    where
        E: ProtocolEncode,
    {
        self.socket.encode(message)
    }

    /// send frontend message to a buffer, this does not write to underlying io
    pub fn send<E>(&mut self, msg: E)
    where
        E: FrontendMessage,
    {
        frontend::write(msg, self.socket.write_buf_mut())
    }

    /// write buffered message to underlying io
    pub fn flush(&mut self) -> impl Future<Output = io::Result<()>> {
        self.socket.flush()
    }

    /// receive a single message
    pub fn recv<D: ProtocolDecode>(&mut self) -> impl Future<Output = Result<D>> {
        self.socket.decode()
    }

    #[cfg(test)]
    #[allow(unused)]
    pub fn debug_read(&mut self) -> impl Future<Output = Result<()>> {
        self.socket.debug_read()
    }
}

