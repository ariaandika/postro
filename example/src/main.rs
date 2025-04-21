use futures::TryStreamExt;

#[tokio::main]
async fn main() -> qs::Result<()> {

    let mut conn = qs::PgConnection::connect("postgres://cookiejar:cookie@127.0.0.1:5432/postgres").await?;

    let _result = qs::query("select 420,'Foo',$1", &mut conn)
        .bind("Deez")
        .fetch_all::<(i32,String,String)>()
        .await?;

    dbg!(_result);

    let _result = qs::query("select 420,'Foo',$1", &mut conn)
        .bind("Deez")
        .fetch_all_v2::<(i32,String,String)>()
        .await?;

    dbg!(_result);

    let mut stream = qs::query("select 420,'Foo',$1", &mut conn)
        .bind("Deez")
        .fetch::<(i32,String,String)>();

    while let Some(_item) = stream.try_next().await? {
        dbg!(_item);
    }

    Ok(())
}
