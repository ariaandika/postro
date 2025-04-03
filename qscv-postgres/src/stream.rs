use std::io;

use super::options::PgOptions;
use crate::{
    error::Result,
    net::{BufferedSocket, Socket},
    protocol::{ProtocolDecode, ProtocolEncode, ProtocolError},
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

