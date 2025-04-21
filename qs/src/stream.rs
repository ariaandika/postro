use bytes::{Buf, BytesMut};
use std::{io, task::{ready, Context, Poll}};

use crate::{
    net::{Socket, WriteAllBuf}, postgres::{frontend, BackendProtocol, ErrorResponse, FrontendProtocol, NoticeResponse}, transport::PgTransport, Error, PgOptions, Result
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
}

impl PgTransport for PgStream {
    type Flush<'a> = WriteAllBuf<'a, BytesMut> where Self: 'a;

    type Recv<'a, B> = Recv<'a, B> where B: BackendProtocol, Self: 'a;

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        crate::io::poll_write_all(&mut self.socket, &mut self.write_buf, cx)
    }

    fn poll_recv<B: BackendProtocol>(&mut self, cx: &mut Context) -> Poll<Result<B>> {
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

    fn send<E>(&mut self, msg: E)
    where
        E: FrontendProtocol,
    {
        frontend::write(msg, &mut self.write_buf);
    }

    fn send_startup(&mut self, msg: frontend::Startup) {
        msg.write(&mut self.write_buf);
    }

    fn flush(&mut self) -> Self::Flush<'_> {
        self.socket.write_all_buf(&mut self.write_buf)
    }

    fn recv<B: BackendProtocol>(&mut self) -> Self::Recv<'_, B> {
        Recv::new(self)
    }

    fn as_pg_stream(&mut self) -> &mut crate::stream::PgStream {
        self
    }
}

pub use recv::Recv;

mod recv {
    use std::{
        marker::PhantomData,
        pin::Pin,
        task::{Context, Poll},
    };

    use super::PgStream;
    use crate::{postgres::backend::NoticeResponse, Result};

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

    #[cfg(feature = "tokio")]
    impl<B> Future for Recv<'_, B>
    where
        B: crate::postgres::BackendProtocol
    {
        type Output = Result<B>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            use std::task::ready;
            use bytes::Buf;
            use crate::{postgres::{backend::ErrorResponse, BackendProtocol}, Error};

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
                    },
                    State::ReadSocket => {
                        ready!(crate::io::poll_read(&mut stream.socket, &mut stream.read_buf, cx)?);
                        *state = State::Read;
                    },
                }
            }
        }
    }

    #[cfg(not(feature = "tokio"))]
    impl<B> Future for Recv<'_, B> {
        type Output = Result<B>;

        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
            let _ = crate::io::poll_read::<crate::net::Socket, bytes::BytesMut>;
            let _ = &self.stream.read_buf;
            let _ = State::ReadSocket;
            panic!("runtime disabled")
        }
    }
}

