use std::{
    io,
    task::{Context, Poll},
};

#[cfg(feature = "tokio")]
pub fn poll_read<R, B>(reader: &mut R, buf: &mut B, cx: &mut Context) -> Poll<io::Result<usize>>
where
    R: tokio::io::AsyncRead + Unpin + ?Sized,
    B: bytes::BufMut + ?Sized,
{
    use std::{pin::Pin, task::ready};
    use tokio::io::ReadBuf;

    if !buf.has_remaining_mut() {
        return Poll::Ready(Ok(0));
    }

    let n = {
        let dst = buf.chunk_mut();
        let dst = unsafe { dst.as_uninit_slice_mut() };
        let mut buf = ReadBuf::uninit(dst);
        let ptr = buf.filled().as_ptr();
        ready!(Pin::new(reader).poll_read(cx, &mut buf)?);

        // Ensure the pointer does not change from under us
        assert_eq!(ptr, buf.filled().as_ptr());
        buf.filled().len()
    };

    // Safety: This is guaranteed to be the number of initialized (and read)
    // bytes due to the invariants provided by `ReadBuf::filled`.
    unsafe {
        buf.advance_mut(n);
    }

    Poll::Ready(Ok(n))
}

#[cfg(feature = "tokio")]
pub fn poll_write_all<W, B>(writer: &mut W, buf: &mut B, cx: &mut Context) -> Poll<io::Result<()>>
where
    W: tokio::io::AsyncWrite + Unpin + ?Sized,
    B: bytes::Buf + ?Sized,
{
    use std::{io::IoSlice, pin::Pin, task::ready};

    const MAX_VECTOR_ELEMENTS: usize = 64;

    while buf.has_remaining() {
        let n = if writer.is_write_vectored() {
            let mut slices = [IoSlice::new(&[]); MAX_VECTOR_ELEMENTS];
            let cnt = buf.chunks_vectored(&mut slices);
            ready!(Pin::new(&mut *writer).poll_write_vectored(cx, &slices[..cnt]))?
        } else {
            ready!(Pin::new(&mut *writer).poll_write(cx, buf.chunk())?)
        };
        buf.advance(n);
        if n == 0 {
            return Poll::Ready(Err(io::ErrorKind::WriteZero.into()));
        }
    }

    Poll::Ready(Ok(()))
}




#[cfg(not(feature = "tokio"))]
pub fn poll_read<R, B>(reader: &mut R, buf: &mut B, cx: &mut Context) -> Poll<io::Result<usize>> {
    let _ = (reader, buf, cx);
    panic!("runtime disabled")
}

#[cfg(not(feature = "tokio"))]
pub fn poll_write_all<W, B>(writer: &mut W, buf: &mut B, cx: &mut Context) -> Poll<io::Result<()>> {
    let _ = (writer, buf, cx);
    panic!("runtime disabled")
}

