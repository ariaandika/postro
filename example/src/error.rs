use postro::{DecodeError, FromRow, Pool, Result, Row, query, query_as};
use tracing::{Instrument, trace_span};

pub async fn main() -> Result<()> {
    let pool = Pool::connect_env().await?;

    let handles = (0..48).map(|i| {
        let pool = pool.clone();
        tokio::spawn(async move {
            if i % 6 == 0 {
                query("SELECT foo", pool).await?;
            } else {
                query_as::<_, _, FailRow>("SELECT 1", pool).fetch_all().await?;
            }
            Ok::<_, postro::Error>(())
        }.instrument(trace_span!("error")))
    });

    for h in handles {
        let _ = h.await.unwrap();
    }

    Ok(())
}

struct FailRow;

impl FromRow for FailRow {
    fn from_row(_: Row) -> Result<Self, DecodeError> {
        Err(DecodeError::IndexOutOfBounds(69))
    }
}
