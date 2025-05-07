use postro::{execute, Pool, Result};
use tracing::{trace_span, Instrument};


pub async fn main() -> Result<()> {
    let pool = Pool::connect_env().await?;

    let handles = (0..48)
        .map(|i|{
            let pool = pool.clone();
            tokio::spawn(
                if i % 6 == 0 {
                    execute("SELECT foo", pool).into_future()
                } else {
                    execute("SELECT 1", pool).into_future()
                }.instrument(trace_span!("error"))
            )
        });

    for h in handles {
        let _ = h.await.unwrap();
    }

    Ok(())
}

