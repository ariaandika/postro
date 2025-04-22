use bytes::{Buf, BytesMut};
use std::{
    io,
    task::{Context, Poll, ready},
};

use crate::{
    Error, PgOptions, Result,
    net::Socket,
    postgres::{BackendProtocol, ErrorResponse, FrontendProtocol, NoticeResponse, frontend},
};

const DEFAULT_BUF_CAPACITY: usize = 1024;

/// Buffered connection to postgres.
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

    pub fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        crate::io::poll_write_all(&mut self.socket, &mut self.write_buf, cx)
    }

    pub fn poll_recv<B: BackendProtocol>(&mut self, cx: &mut Context) -> Poll<Result<B>> {
        loop {
            let Some(mut header) = self.read_buf.get(..5) else {
                self.read_buf.reserve(1024);
                ready!(crate::io::poll_read(&mut self.socket, &mut self.read_buf, cx)?);
                continue;
            };

            let msgtype = header.get_u8();
            let len = header.get_i32() as _;

            if self.read_buf.len() - 1/*msgtype*/ < len {
                self.read_buf.reserve(1 + len);
                ready!(crate::io::poll_read(&mut self.socket, &mut self.read_buf, cx)?);
                continue;
            }

            self.read_buf.advance(5);
            let body = self.read_buf.split_to(len - 4).freeze();

            let res = match msgtype {
                ErrorResponse::MSGTYPE => {
                    let err = ErrorResponse::decode(msgtype, body).unwrap();
                    Err(Error::Database(err))?
                }
                NoticeResponse::MSGTYPE => {
                    todo!()
                }
                _ => B::decode(msgtype, body)?,
            };

            return Poll::Ready(Ok(res));
        }
    }

    pub fn send<E>(&mut self, msg: E)
    where
        E: FrontendProtocol,
    {
        frontend::write(msg, &mut self.write_buf);
    }

    pub fn send_startup(&mut self, msg: frontend::Startup) {
        msg.write(&mut self.write_buf);
    }
}

