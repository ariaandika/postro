use std::{
    io,
    marker::PhantomPinned,
    pin::Pin,
    task::{Context, Poll},
};

pin_project_lite::pin_project! {
    /// A future to write some of the buffer to an `AsyncWrite`.
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct WriteAllBuf<'a, W, B> {
        writer: &'a mut W,
        buf: &'a mut B,
        #[pin]
        _pin: PhantomPinned,
    }
}

impl<'a, W, B> WriteAllBuf<'a, W, B> {
    pub fn new(writer: &'a mut W, buf: &'a mut B) -> Self {
        Self { writer, buf, _pin: PhantomPinned }
    }
}

#[cfg(not(feature = "tokio"))]
impl<W, B> Future for WriteAllBuf<'_, W, B> {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        panic!("runtime disabled")
    }
}

#[cfg(feature = "tokio")]
impl<W, B> Future for WriteAllBuf<'_, W, B>
where
    W: tokio::io::AsyncWrite + Unpin,
    B: bytes::Buf,
{
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        use std::{io::IoSlice, task::ready};
        use tokio::io::AsyncWrite;

        const MAX_VECTOR_ELEMENTS: usize = 64;

        let me = self.project();
        while me.buf.has_remaining() {
            let n = if me.writer.is_write_vectored() {
                let mut slices = [IoSlice::new(&[]); MAX_VECTOR_ELEMENTS];
                let cnt = me.buf.chunks_vectored(&mut slices);
                ready!(Pin::new(&mut *me.writer).poll_write_vectored(cx, &slices[..cnt]))?
            } else {
                ready!(Pin::new(&mut *me.writer).poll_write(cx, me.buf.chunk())?)
            };
            me.buf.advance(n);
            if n == 0 {
                return Poll::Ready(Err(io::ErrorKind::WriteZero.into()));
            }
        }

        Poll::Ready(Ok(()))
    }
}

