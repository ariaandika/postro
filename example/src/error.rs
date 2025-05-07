use postro::{DecodeError, FromRow, Pool, Result, Row, execute, query};
use tracing::{Instrument, trace_span};

pub async fn main() -> Result<()> {
    let pool = Pool::connect_env().await?;

    let handles = (0..48).map(|i| {
        let pool = pool.clone();
        tokio::spawn(
            if i % 6 == 0 {
                execute("SELECT foo", pool).into_future()
            } else {
                query::<_, _, FailRow>("SELECT 1", pool).into_future()
            }
            .instrument(trace_span!("error")),
        )
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
