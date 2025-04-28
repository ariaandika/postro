use std::env::var;
use futures::TryStreamExt;
use qs::{transaction::Transaction, Result};
use tracing::Instrument;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let mut conn = qs::PgConnection::connect(&var("DATABASE_URL").unwrap()).await?;

    qs::execute("create temp table post(id serial, name text)", &mut conn)
        .execute()
        .await?;

    qs::query::simple_query::<(), _>("select * from post", &mut conn)
        .instrument(tracing::trace_span!("simple query"))
        .await?;

    let _err = qs::query::<_, _, ()>("select deez", &mut conn)
        .fetch_one()
        .await
        .unwrap_err();

    conn.healthcheck().await?;

    async {
        let mut tx = Transaction::begin(&mut conn).await?;
        qs::execute("insert into post(name) values('Deez2')", &mut tx)
            .execute()
            .await?;
        Ok::<_, qs::Error>(())
    }.instrument(tracing::trace_span!("transaction")).await?;

    let posts = qs::query::<_, _, (i32, String)>("select * from post", &mut conn)
        .fetch_all()
        .await?;

    assert!(posts.is_empty());

    let (id,): (i32,) = qs::query("insert into post(name) values($1) returning id", &mut conn)
        .bind("Foo")
        .fetch_one()
        .instrument(tracing::trace_span!("inserter"))
        .await?;

    let post = qs::query::<_, _, (i32, String)>("select * from post", &mut conn)
        .fetch_all()
        .await?;

    assert_eq!(post[0].0, id);

    qs::execute("insert into post(name) values($1)", &mut conn)
        .bind("Deez")
        .execute()
        .await?;

    let mut stream = qs::query::<_, _, (i32, String)>("select * from post", &mut conn).fetch();

    let p1 = stream.try_next().await?.unwrap();
    assert_eq!(p1.0, id);
    assert_eq!(p1.1.as_str(), "Foo");

    let p2 = stream.try_next().await?.unwrap();
    assert_eq!(p2.1.as_str(), "Deez");

    assert!(stream.try_next().await?.is_none());

    {
        let mut tx = Transaction::begin(&mut conn).await?;
        qs::execute("insert into post(name) values('Deez2')", &mut tx)
            .execute()
            .await?;
        tx.commit().await?;
    }

    let posts = qs::query::<_, _, (i32, String)>("select * from post", &mut conn)
        .fetch_all()
        .await?;

    assert!(posts.iter().find(|e|matches!(e.1.as_str(),"Deez2")).is_some());

    for _ in 0..2 {
        let _ok = qs::query::<_, _, ()>("Select * from post", &mut conn)
            .fetch_one()
            .await?;
        let _ok = qs::query::<_, _, ()>("sElect * from post", &mut conn)
            .fetch_one()
            .await?;
        let _ok = qs::query::<_, _, ()>("seLect * from post", &mut conn)
            .fetch_one()
            .await?;
    }

    Ok(())
}
