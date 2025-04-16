use std::{
    io,
    marker::PhantomPinned,
    pin::Pin,
    task::{Context, Poll},
};

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct ReadBuf<'a, R: ?Sized, B: ?Sized> {
        reader: &'a mut R,
        buf: &'a mut B,
        #[pin]
        _pin: PhantomPinned,
    }
}

impl<'a, R: ?Sized, B: ?Sized> ReadBuf<'a, R, B> {
    pub fn new(reader: &'a mut R, buf: &'a mut B) -> Self {
        Self { reader, buf, _pin: PhantomPinned }
    }
}

#[cfg(not(feature = "tokio"))]
impl<R, B> Future for ReadBuf<'_, R, B> {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        panic!("runtime disabled")
    }
}

#[cfg(feature = "tokio")]
impl<R, B> Future for ReadBuf<'_, R, B>
where
    R: tokio::io::AsyncRead + Unpin + ?Sized,
    B: bytes::BufMut + ?Sized,
{
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
        use std::{
            pin::Pin,
            task::{Poll, ready},
        };
        use tokio::io::ReadBuf;

        let me = self.project();

        if !me.buf.has_remaining_mut() {
            return Poll::Ready(Ok(0));
        }

        let n = {
            let dst = me.buf.chunk_mut();
            let dst = unsafe { dst.as_uninit_slice_mut() };
            let mut buf = ReadBuf::uninit(dst);
            let ptr = buf.filled().as_ptr();
            ready!(tokio::io::AsyncRead::poll_read(Pin::new(me.reader), cx, &mut buf)?);

            // Ensure the pointer does not change from under us
            assert_eq!(ptr, buf.filled().as_ptr());
            buf.filled().len()
        };

        // Safety: This is guaranteed to be the number of initialized (and read)
        // bytes due to the invariants provided by `ReadBuf::filled`.
        unsafe {
            me.buf.advance_mut(n);
        }

        Poll::Ready(Ok(n))
    }
}

