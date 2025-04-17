


#[tokio::main]
async fn main() -> qs::Result<()> {

    let mut conn = qs::PgConnection::connect("postgres://cookiejar:cookie@127.0.0.1:5432/postgres").await?;

    let _result = qs::query("select 420,'Foo',$1", &mut conn)
        .bind("Deez")
        .fetch_all::<()>()
        .await?;

    dbg!(_result);


    Ok(())
}
