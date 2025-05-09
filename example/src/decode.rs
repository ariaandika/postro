use postro::{query_as, types::Json, Connection, Result};
use serde::Deserialize;
use time::{PrimitiveDateTime, UtcDateTime};

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Foo {
    id: i32,
}

pub async fn main() -> Result<()> {
    let mut conn = Connection::connect_env().await?;

    let (null,): (Option<String>,) = query_as("SELECT NULL::TEXT", &mut conn).fetch_one().await?;

    assert!(null.is_none());

    // `time`

    let now_utc = UtcDateTime::now().replace_millisecond(0).unwrap();
    let (local, utc): (PrimitiveDateTime, UtcDateTime) =
        query_as("SELECT now()::TIMESTAMP,now()::TIMESTAMPTZ", &mut conn)
            .fetch_one()
            .await?;

    assert_eq!(
        (local.month(), local.minute(), local.second()),
        (now_utc.month(), now_utc.minute(), now_utc.second()),
    );
    assert_eq!(utc.replace_millisecond(0).unwrap(), now_utc);

    // `time`

    let (Json(json),): (Json<Foo>,) = query_as("SELECT '{\"id\":420}'::jsonb", &mut conn)
        .fetch_one()
        .await?;

    assert_eq!(json, Foo { id: 420 });

    Ok(())
}

