use crate::error::Result;

#[derive(Debug)]
pub enum Socket {
    #[cfg(feature = "tokio")]
    TokioTcp(tokio::net::TcpStream),
    #[cfg(all(feature = "tokio", unix))]
    TokioSocket(tokio::net::UnixStream),
}

impl Socket {
    pub async fn connect_tcp(host: &str, port: u16) -> Result<Socket> {
        #[cfg(feature = "tokio")]
        {
            let socket = tokio::net::TcpStream::connect((host,port)).await?;
            socket.set_nodelay(true)?;
            Ok(Socket::TokioTcp(socket))
        }

        #[cfg(not(feature = "tokio"))]
        {
            let _ = (host,port);
            panic!("runtime disabled")
        }
    }

    pub async fn connect_socket(path: &str) -> Result<Socket> {
        #[cfg(feature = "tokio")]
        {
            let socket = tokio::net::UnixStream::connect(path).await?;
            Ok(Socket::TokioSocket(socket))
        }

        #[cfg(not(feature = "tokio"))]
        {
            let _ = path;
            panic!("runtime disabled")
        }
    }
}



