use postro::{query, types::Json, Connection, Result};
use serde::Deserialize;
use time::{PrimitiveDateTime, UtcDateTime};

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Foo {
    id: i32,
}

pub async fn main() -> Result<()> {
    let mut conn = Connection::connect_env().await?;

    let now_utc = UtcDateTime::now().replace_millisecond(0).unwrap();
    let (local, utc) = query::<_, _, (PrimitiveDateTime, UtcDateTime)>(
        "SELECT now()::TIMESTAMP,now()::TIMESTAMPTZ",
        &mut conn,
    )
    .fetch_all()
    .await?[0];

    assert_eq!(
        (local.month(), local.day(), local.minute(), local.second()),
        (
            now_utc.month(),
            now_utc.day(),
            now_utc.minute(),
            now_utc.second()
        ),
    );
    assert_eq!(utc.replace_millisecond(0).unwrap(), now_utc);

    let app = query::<_, _, (Json<Foo>,)>("SELECT '{\"id\":420}'::jsonb", &mut conn)
        .fetch_all()
        .await?;

    assert_eq!(app[0].0.0, Foo { id: 420 });

    Ok(())
}

