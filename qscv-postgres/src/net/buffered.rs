use bytes::BytesMut;
use std::{io, ops::ControlFlow};

use super::Socket;
use crate::{
    protocol::{ProtocolDecode, ProtocolEncode, ProtocolError},
    Result,
};

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

    /// write message to a buffer, this does not write to underlying io
    pub fn encode<E: ProtocolEncode>(&mut self, message: E) -> Result<(), ProtocolError> {
        message.encode(&mut self.write_buf)
    }

    /// write buffered message to underlying io
    pub fn flush(&mut self) -> impl Future<Output = io::Result<()>> {
        self.socket.write_all_buf(&mut self.write_buf)
    }

    /// read message from socket
    pub async fn decode<D: ProtocolDecode>(&mut self) -> Result<D> {
        loop {
            let read = self.read_buf.split().freeze();
            let mut decode = read.clone();
            match D::decode(&mut decode)? {
                ControlFlow::Continue(expect) => {
                    drop(decode);
                    let Ok(bytes) = read.try_into_mut() else {
                        panic!("Decoding violation: bytes owned before decode finish");
                    };
                    self.read_buf.unsplit(bytes);
                    loop {
                        self.socket.read_buf(&mut self.read_buf).await?;
                        if self.read_buf.len() >= expect {
                            break
                        }
                    }
                },
                ControlFlow::Break(m) => {
                    let Ok(ok) = decode.try_into_mut() else {
                        panic!("Decoding violation: bytes should be split, instead of cloned");
                    };
                    self.read_buf.unsplit(ok);
                    return Ok(m)
                }
            }
        }
    }

    pub async fn debug_read(&mut self) {
        self.read_buf.clear();
        self.socket.read_buf(&mut self.read_buf).await.unwrap();
        dbg!(&self.read_buf);
    }
}

