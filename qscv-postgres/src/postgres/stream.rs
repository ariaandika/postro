use super::{message::FrontendMessage, options::PgOptions};
use crate::{error::Result, net::{BufferedSocket, Socket}};

#[derive(Debug)]
pub struct PgStream {
    socket: BufferedSocket,
}

impl PgStream {
    pub async fn connect(opt: &PgOptions) -> Result<Self> {
        let socket = match &*opt.host {
            "localhost" => Socket::connect_socket(&(format!("/run/postgresql/.s.PGSQL.{}",opt.port))).await?,
            _ => Socket::connect_tcp(&opt.host, opt.port).await?,
        };

        Ok(Self { socket: BufferedSocket::new(socket) })
    }

    pub fn write_msg(&mut self, message: FrontendMessage) -> impl Future<Output = Result<()>> {
        self.socket.encode(message)
    }

    pub fn write(&mut self, message: impl Into<FrontendMessage>) -> impl Future<Output = Result<()>> {
        self.write_msg(message.into())
    }

    pub async fn debug_read(&mut self) {
        self.socket.debug_read().await;
    }
}

