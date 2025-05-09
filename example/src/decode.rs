use postro::{Connection, Decode, Encode, Result, query, query_as, query_scalar, types::Json};
use serde::Deserialize;
use time::{PrimitiveDateTime, UtcDateTime};

#[derive(Decode, Encode)]
struct MyId(i32);

#[derive(Encode)]
struct MyId2<'a>(&'a str);

#[derive(Decode)]
struct SomeId<T>(T);

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Foo {
    id: i32,
}

pub async fn main() -> Result<()> {
    let mut conn = Connection::connect_env().await?;

    let my_id = MyId(420);

    let _ = query("", &mut conn).bind(&my_id);

    let (null,): (Option<String>,) = query_as("SELECT NULL::TEXT", &mut conn).fetch_one().await?;

    assert!(null.is_none());

    let my_id: MyId = query_scalar("SELECT 420", &mut conn).fetch_one().await?;

    assert_eq!(my_id.0, 420);

    let some_id: SomeId<i32> = query_scalar("SELECT 420", &mut conn).fetch_one().await?;

    assert_eq!(some_id.0, 420);

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

