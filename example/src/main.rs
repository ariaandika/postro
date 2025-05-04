use futures::TryStreamExt;
use std::env::var;
use tracing::{Instrument, trace_span};
use tracing_subscriber::{
    EnvFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

use postro::{DecodeError, Executor, FromRow, Pool, Result};

#[derive(Debug, FromRow)]
struct Post {
    #[allow(unused)]
    id: i32,
    name: String,
    tag: String,
}

#[derive(Debug, FromRow)]
#[allow(unused)]
struct PostTuple(i32, String, String);

const SLEEP_MUL: i32 = 0;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::Registry::default()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer()
            .with_target(false))
        .init();

    let url = var("DATABASE_URL").unwrap();

    {
        let mut conn = postro::Connection::connect(&url).await?;
        postro::execute("drop table if exists post", &mut conn).execute().await?;
        postro::execute("create table post(id serial, tag text, name text)", &mut conn).execute().await?;
        task(&mut conn, 0)
            .instrument(trace_span!("dedicated"))
            .await?;
    }

    let mut pool = Pool::connect_lazy(&url)?;
    let mut handles = vec![];

    doc_example(pool.clone()).instrument(trace_span!("doc_example")).await?;

    for i in 0..24i32  {
        tokio::time::sleep(std::time::Duration::from_millis(
            (i * 100 / 4 * SLEEP_MUL) as _,
        ))
        .await;
        handles.push(tokio::spawn(
            task(pool.clone(), i.into()).instrument(trace_span!("thread", thread = i)),
        ));
    }

    for handle in std::mem::take(&mut handles) {
        handle.await.unwrap()?;
    }

    let _foo: Vec<Post> = postro::query("select * from post", &mut pool).fetch_all().await?;

    // tracing::info!("{foo:#?}");

    // tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    Ok(())
}

async fn doc_example(mut pool: Pool) -> Result<()> {
    let res = postro::query::<_, _, (i32,String)>("SELECT 420,$1", &mut pool)
        .bind("Foo")
        .fetch_one()
        .await?;

    assert_eq!(res.0,420);
    assert_eq!(res.1.as_str(),"Foo");

    // ======

    postro::execute("DROP TABLE IF EXISTS foo", &mut pool).execute().await?;
    postro::execute("CREATE TABLE IF NOT EXISTS foo(id int)", &mut pool)
        .execute()
        .await?;

    let mut handles = vec![];

    for i in 0..14 {
        let pool = pool.clone();
        let t = tokio::spawn(async move {
            postro::execute("INSERT INTO foo(id) VALUES($1)", &pool)
                .bind(i)
                .execute()
                .await
        }.instrument(trace_span!("doc_example")));
        handles.push(t);
    }

    for h in handles {
        h.await.unwrap()?;
    }

    let foos = postro::query::<_, _, (i32,)>("SELECT * FROM foo", &mut pool)
        .fetch_all()
        .await?;

    assert_eq!(foos.len(), 14);

    Ok(())
}

async fn task<E: Executor>(conn: E, id: i32) -> Result<()> {
    tracing::trace!("task");

    let mut conn = conn.connection().await?;

    postro::query::simple_query::<(), _>("select * from post", &mut conn)
        .instrument(tracing::trace_span!("simple query"))
        .await?;

    let err = postro::execute("select deez", &mut conn)
        .fetch_one()
        .await
        .unwrap_err();

    tracing::error!("Expected Error: {err}");

    {
        let mut tx = postro::query::begin(&mut conn).await?;
        postro::execute("insert into post(tag,name) values($1,$2)", &mut tx)
            .bind(&format!("thead{id}"))
            .bind(&format!("NotExists: thread{id}"))
            .execute()
            .await?;
    }

    let (_post_id,) = postro::query::<_, _, (i32,)>("insert into post(tag,name) values($1,$2) returning id", &mut conn)
        .bind(&format!("thead{id}"))
        .bind(&format!("Post from: thread{id}"))
        .fetch_one()
        .await?;

    let post = postro::query::<_, _, Post>("select * from post", &mut conn)
        .fetch_all()
        .await?;

    assert!(
        post.iter()
            .find(|e| e.tag == format!("thead{id}"))
            .map(|e| e.name == format!("Post from: thread{id}"))
            .unwrap()
    );

    postro::execute("insert into post(tag,name) values($1,$2)", &mut conn)
        .bind(&format!("thead{id}"))
        .bind(&format!("Exectute for: thread{id}"))
        .execute()
        .await?;

    let mut stream = postro::query::<_, _, Post>("select * from post", &mut conn).fetch();

    while let Some(post) = stream.try_next().await? {
        let _ = post;
    }

    {
        let mut tx = postro::query::begin(&mut conn).await?;
        postro::execute("insert into post(tag,name) values($1,$2)", &mut tx)
            .bind(&format!("thead{id}"))
            .bind(&format!("Transaction from: thread{id}"))
            .execute()
            .await?;
        tx.commit().await?;
    }

    tokio::time::sleep(std::time::Duration::from_millis((id*100/2 * SLEEP_MUL) as _)).await;

    Ok(())
}
