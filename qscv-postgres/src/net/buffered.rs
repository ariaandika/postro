use bytes::BytesMut;
use std::ops::ControlFlow;

use super::Socket;
use crate::{protocol::{ProtocolDecode, ProtocolEncode}, Result};

const DEFAULT_BUF_CAPACITY: usize = 1024;

/// buffered read and write socket
#[derive(Debug)]
pub struct BufferedSocket {
    socket: Socket,
    read_buf: BytesMut,
    write_buf: BytesMut,
}

impl BufferedSocket {
    pub fn new(socket: Socket) -> Self {
        Self {
            socket,
            read_buf: BytesMut::with_capacity(DEFAULT_BUF_CAPACITY),
            write_buf: BytesMut::with_capacity(DEFAULT_BUF_CAPACITY),
        }
    }

    pub async fn decode<D: ProtocolDecode>(&mut self) -> Result<D> {
        let len = loop {
            match D::check(&self.read_buf)? {
                ControlFlow::Continue(expect) => loop {
                    self.socket.read_buf(&mut self.read_buf).await?;
                    if self.read_buf.len() >= expect {
                        break
                    }
                },
                ControlFlow::Break(len) => break len,
            }
        };

        return Ok(D::consume(self.read_buf.split_to(len).freeze())?)
    }

    /// write message to a buffer, this does not write to underlying io
    pub fn encode<E: ProtocolEncode>(&mut self, message: E) -> Result<()> {
        message.write(&mut self.write_buf).map_err(Into::into)
    }

    /// write buffered message to underlying io
    pub async fn flush(&mut self) -> Result<()> {
        self.socket.write_buf(&mut self.write_buf).await
    }

    pub async fn debug_read(&mut self) {
        self.read_buf.clear();
        self.socket.read_buf(&mut self.read_buf).await.unwrap();
        dbg!(&self.read_buf);
    }
}

