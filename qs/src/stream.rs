use bytes::{Buf, BytesMut};

use crate::{
    PgOptions, Result,
    io::WriteAllBuf,
    message::{
        FrontendProtocol,
        backend::BackendProtocol,
        frontend::{self, Startup},
    },
    net::Socket,
};

const DEFAULT_BUF_CAPACITY: usize = 1024;

#[derive(Debug)]
pub struct PgStream {
    socket: Socket,
    read_buf: BytesMut,
    write_buf: BytesMut,
}

impl PgStream {
    pub async fn connect(opt: &PgOptions) -> Result<Self> {
        let socket = match &*opt.host {
            "localhost" => Socket::connect_socket(&(format!("/run/postgresql/.s.PGSQL.{}",opt.port))).await?,
            _ => Socket::connect_tcp(&opt.host, opt.port).await?,
        };

        Ok(Self {
            socket,
            read_buf: BytesMut::with_capacity(DEFAULT_BUF_CAPACITY),
            write_buf: BytesMut::with_capacity(DEFAULT_BUF_CAPACITY),
        })
    }

    pub fn send_startup(&mut self, msg: Startup) {
        msg.write(&mut self.write_buf);
    }

    /// send frontend message to a buffer
    ///
    /// just calling this function, msg only written to a buffer
    ///
    /// polling the returned `Flush` will actually flush the underlying io
    pub fn send<E>(&mut self, msg: E)
    where
        E: FrontendProtocol,
    {
        frontend::write(msg, &mut self.write_buf);
    }

    /// write buffered message to underlying io
    pub fn flush(&mut self) -> WriteAllBuf<'_, Socket, BytesMut> {
        self.socket.write_all_buf(&mut self.write_buf)
    }

    /// receive a single message
    pub async fn recv<B: BackendProtocol>(&mut self) -> Result<B> {
        loop {
            let Some(mut header) = self.read_buf.get(..5) else {
                self.read_buf.reserve(1024);
                self.socket.read_buf(&mut self.read_buf).await?;
                continue;
            };

            let msgtype = header.get_u8();
            let len = header.get_i32() as _;

            if self.read_buf.len() - 1/*msgtype*/ < len {
                self.read_buf.reserve(1 + len);
                self.socket.read_buf(&mut self.read_buf).await?;
                continue;
            }

            self.read_buf.advance(5);
            let body = self.read_buf.split_to(len - 4).freeze();

            let msg = B::decode(msgtype, body)?;

            return Ok(msg)
        }
    }
}

