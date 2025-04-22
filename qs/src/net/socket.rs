use std::io;

/// an either `TcpStream` or `Socket`, which implement
/// `AsyncRead` and `AsyncWrite` transparently
///
/// require `tokio` feature, otherwise panic at runtime
pub struct Socket {
    kind: Kind,
}

enum Kind {
    #[cfg(feature = "tokio")]
    TokioTcp(tokio::net::TcpStream),
    #[cfg(all(feature = "tokio", unix))]
    TokioUnixSocket(tokio::net::UnixStream),
}

impl Socket {
    pub async fn connect_tcp(host: &str, port: u16) -> io::Result<Socket> {
        #[cfg(feature = "tokio")]
        {
            let socket = tokio::net::TcpStream::connect((host,port)).await?;
            socket.set_nodelay(true)?;
            Ok(Socket { kind: Kind::TokioTcp(socket) })
        }

        #[cfg(not(feature = "tokio"))]
        {
            let _ = (host,port);
            panic!("runtime disabled")
        }
    }

    pub async fn connect_socket(path: &str) -> io::Result<Socket> {
        #[cfg(all(feature = "tokio", unix))]
        {
            let socket = tokio::net::UnixStream::connect(path).await?;
            Ok(Socket { kind: Kind::TokioUnixSocket(socket) })
        }

        #[cfg(not(all(feature = "tokio", unix)))]
        {
            let _ = path;
            panic!("runtime disabled")
        }
    }
}

#[cfg(feature = "tokio")]
impl tokio::io::AsyncRead for Socket {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        use std::pin::Pin;
        match &mut self.kind {
            Kind::TokioTcp(t) => Pin::new(t).poll_read(cx, buf),
            #[cfg(unix)]
            Kind::TokioUnixSocket(u) => Pin::new(u).poll_read(cx, buf),
        }
    }
}

#[cfg(feature = "tokio")]
impl tokio::io::AsyncWrite for Socket {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
        use std::pin::Pin;
        match &mut self.kind {
            Kind::TokioTcp(t) => Pin::new(t).poll_write(cx, buf),
            #[cfg(unix)]
            Kind::TokioUnixSocket(u) => Pin::new(u).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        use std::pin::Pin;
        match &mut self.kind {
            Kind::TokioTcp(t) => Pin::new(t).poll_flush(cx),
            #[cfg(unix)]
            Kind::TokioUnixSocket(u) => Pin::new(u).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        use std::pin::Pin;
        match &mut self.kind {
            Kind::TokioTcp(t) => Pin::new(t).poll_shutdown(cx),
            #[cfg(unix)]
            Kind::TokioUnixSocket(u) => Pin::new(u).poll_shutdown(cx),
        }
    }
}

impl std::fmt::Debug for Socket {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            #[cfg(feature = "tokio")]
            Kind::TokioTcp(ref tcp) => std::fmt::Debug::fmt(tcp, _f),
            #[cfg(all(feature = "tokio", unix))]
            Kind::TokioUnixSocket(ref unix) => std::fmt::Debug::fmt(&unix, _f),
        }
    }
}

