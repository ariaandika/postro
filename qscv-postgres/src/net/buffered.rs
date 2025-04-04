use bytes::BytesMut;
use std::{io, ops::ControlFlow};

use super::Socket;
use crate::{
    protocol::ProtocolDecode,
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

    /// write buffered message to underlying io
    pub fn flush(&mut self) -> impl Future<Output = io::Result<()>> {
        self.socket.write_all_buf(&mut self.write_buf)
    }

    /// read message from socket
    //
    // Case 1:
    // to prevent copies, we can use `Bytes` to share memory
    // but the reader `BytesMut` cannot reclaim that memory back
    //
    // Case 2:
    // we can just give a slice, and decoder msu copy the required bytes,
    // but the reader buffer cannot know how much bytes was read
    //
    // Case 3:
    // we can give the entire mutable reference of the `BytesMut`, the decoder
    // will detect is the amount of bytes is sufficient, if its not, decoder should
    // not modify the `BytesMut` in anyway, finally, decoder can split the required
    // `Bytes`, and the reader have the leftover bytes
    pub async fn decode<D: ProtocolDecode>(&mut self) -> Result<D> {
        loop {
            #[cfg(debug_assertions)]
            let prev = (self.read_buf.len(),self.read_buf.capacity());
            match D::decode(&mut self.read_buf)? {
                ControlFlow::Continue(expect) => {
                    debug_assert_eq!(prev,(self.read_buf.len(),self.read_buf.capacity()));
                    self.read_buf.reserve(expect.saturating_sub(self.read_buf.len()));
                    loop {
                        if self.read_buf.len() >= expect {
                            break
                        }
                        self.socket.read_buf(&mut self.read_buf).await?;
                    }
                },
                ControlFlow::Break(m) => return Ok(m),
            }
        }
    }

    #[cfg(test)]
    #[allow(unused)]
    pub async fn debug_read(&mut self) -> Result<()> {
        println!("Debug Read: {:?}",self.read_buf);
        self.socket.read_buf(&mut self.read_buf).await?;
        println!("Debug PostRead: {:?}",self.read_buf);
        Ok(())
    }

    /// return mutable reference to the write buffer
    pub fn write_buf_mut(&mut self) -> &mut BytesMut {
        &mut self.write_buf
    }
}

