use bytes::BytesMut;

use crate::{
    PgOptions, Result,
    net::WriteAllBuf,
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
    pub fn flush(&mut self) -> WriteAllBuf<'_, BytesMut> {
        self.socket.write_all_buf(&mut self.write_buf)
    }

    /// receive a single message
    pub fn recv<B: BackendProtocol>(&mut self) -> Recv<B> {
        Recv::new(self)
    }

}

// pub async fn recv<B: BackendProtocol>(&mut self) -> Result<B> {
//     loop {
//         let Some(mut header) = self.read_buf.get(..5) else {
//             self.read_buf.reserve(1024);
//             self.socket.read_buf(&mut self.read_buf).await?;
//             continue;
//         };
//
//         let msgtype = header.get_u8();
//         let len = header.get_i32() as _;
//
//         if self.read_buf.len() - 1/*msgtype*/ < len {
//             self.read_buf.reserve(1 + len);
//             self.socket.read_buf(&mut self.read_buf).await?;
//             continue;
//         }
//
//         self.read_buf.advance(5);
//         let body = self.read_buf.split_to(len - 4).freeze();
//
//         let msg = B::decode(msgtype, body)?;
//
//         return Ok(msg)
//     }
// }

pub use recv::Recv;

mod recv {
    use std::{
        marker::PhantomData,
        pin::Pin,
        task::{Context, Poll},
    };

    use super::PgStream;
    use crate::Result;

    pin_project_lite::pin_project! {
        #[derive(Debug)]
        #[project = RecvProject]
        pub struct Recv<'s, B> {
            stream: &'s mut PgStream,
            state: State,
            _p: PhantomData<B>,
        }
    }

    #[derive(Debug)]
    enum State {
        Read,
        ReadSocket,
    }

    impl<'s, B> Recv<'s, B> {
        pub fn new(stream: &'s mut PgStream) -> Self {
            Self { stream, state: State::Read, _p: PhantomData }
        }
    }

    #[cfg(not(feature = "tokio"))]
    impl<'s, B> Future for Recv<'s, B> {
        type Output = Result<B>;

        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
            let _ = &self.stream.read_buf;
            let _ = State::ReadSocket;
            panic!("runtime disabled")
        }
    }

    #[cfg(feature = "tokio")]
    impl<'s, B> Future for Recv<'s, B>
    where
        B: crate::message::BackendProtocol
    {
        type Output = Result<B>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            use std::task::ready;
            use bytes::{Buf, BufMut};
            use tokio::io::{AsyncRead, ReadBuf};

            let RecvProject { stream, state, .. } = self.as_mut().project();

            loop {
                match state {
                    State::Read => {
                        let Some(mut header) = stream.read_buf.get(..5) else {
                            stream.read_buf.reserve(1024);
                            *state = State::ReadSocket;
                            continue;
                        };

                        let msgtype = header.get_u8();
                        let len = header.get_i32() as _;

                        if stream.read_buf.len() - 1/*msgtype*/ < len {
                            stream.read_buf.reserve(1 + len);
                            *state = State::ReadSocket;
                            continue;
                        }

                        stream.read_buf.advance(5);
                        let body = stream.read_buf.split_to(len - 4).freeze();

                        let msg = B::decode(msgtype, body)?;

                        return Poll::Ready(Ok(msg));
                    },
                    State::ReadSocket => {
                        let n = {
                            let dst = stream.read_buf.chunk_mut();
                            let dst = unsafe { dst.as_uninit_slice_mut() };
                            let mut buf = ReadBuf::uninit(dst);
                            let ptr = buf.filled().as_ptr();
                            ready!(Pin::new(&mut stream.socket).poll_read(cx, &mut buf)?);

                            // Ensure the pointer does not change from under us
                            assert_eq!(ptr, buf.filled().as_ptr());
                            buf.filled().len()
                        };

                        // Safety: This is guaranteed to be the number of initialized (and read)
                        // bytes due to the invariants provided by `ReadBuf::filled`.
                        unsafe {
                            stream.read_buf.advance_mut(n);
                        }

                        *state = State::Read;
                    },
                }
            }
        }
    }

    // impl<'pgstream, 'a, B> Future for Recv<'pgstream, 'a, B>
    // where
    //     B: BackendProtocol
    // {
    //     type Output = Result<B>;
    //
    //     fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    //         use std::mem;
    //
    //         loop {
    //             let foo = mem::take(&mut self.as_mut().project());
    //             match foo {
    //                 RecvProject::Read { stream } => {
    //                     let Some(mut header) = stream.read_buf.get(..5) else {
    //                         stream.read_buf.reserve(1024);
    //                         let f = stream.socket.read_buf(&mut stream.read_buf);
    //                         self.set(Self::ReadBuf { stream, f });
    //                         continue;
    //                     };
    //
    //                     todo!()
    //                 }
    //                 RecvProject::ReadBuf { stream, f } => todo!(),
    //                 _ => todo!()
    //             }
    //
    //             // let msgtype = header.get_u8();
    //             // let len = header.get_i32() as _;
    //             //
    //             // if self.read_buf.len() - 1/*msgtype*/ < len {
    //             //     self.read_buf.reserve(1 + len);
    //             //     self.socket.read_buf(&mut self.read_buf).await?;
    //             //     continue;
    //             // }
    //             //
    //             // self.read_buf.advance(5);
    //             // let body = self.read_buf.split_to(len - 4).freeze();
    //             //
    //             // let msg = B::decode(msgtype, body)?;
    //             //
    //             // return Ok(msg)
    //         }
    //     }
    // }
}


