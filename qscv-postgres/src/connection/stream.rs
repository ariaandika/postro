use crate::{error::Result, options::PgOptions, rt::Socket};


#[derive(Debug)]
pub struct PgStream {
    socket: Socket
}

impl PgStream {
    pub async fn connect(opt: &PgOptions) -> Result<Self> {
        let socket = match &*opt.host {
            "localhost" => Socket::connect_socket(&format!("/run/postgres/.psql"/*TODO*/)).await?,
            _ => Socket::connect_tcp(&opt.host, opt.port).await?,
        };

        Ok(Self {
            socket,
        })
    }
}

