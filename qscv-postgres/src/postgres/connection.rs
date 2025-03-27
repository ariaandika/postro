use super::{message::messages::Startup, options::PgOptions, stream::PgStream};
use crate::error::Result;

pub struct PgConnection {
    #[allow(unused)]
    stream: PgStream,
}

impl PgConnection {
    pub async fn connect(url: &str) -> Result<Self> {
        let opt = PgOptions::parse(url)?;
        let mut stream = PgStream::connect(&opt).await?;

        // https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-START-UP

        stream.write(Startup { user: opt.user.bytes() })?;

        stream.flush().await?;

        stream.debug_read().await;

        // LATEST:

        Ok(Self { stream })
    }
}

#[cfg(feature = "tokio")]
#[test]
fn test_connect() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _conn = PgConnection::connect("postgres://postgres:@localhost:5432/deuzo").await.unwrap();
        })
}

const J: u8 = b'R';

