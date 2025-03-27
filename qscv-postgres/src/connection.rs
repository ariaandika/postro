use stream::PgStream;

use crate::{error::Result, options::PgOptions};

mod stream;

pub struct PgConnection {
    #[allow(unused)]
    stream: PgStream,
}

impl PgConnection {
    pub async fn connect(url: &str) -> Result<Self> {
        let opt = PgOptions::parse(url)?;
        let stream = PgStream::connect(&opt).await?;

        Ok(Self {
            stream,
        })
    }

}


