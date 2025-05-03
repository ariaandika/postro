use futures::TryStreamExt;
use std::{borrow::Cow, env::var};
use tracing::Instrument;

use postro::{Connection, DecodeError, Executor, FromRow, Pool, Result};

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

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let url = var("DATABASE_URL").unwrap();

    {
        let mut conn = Connection::connect(&url).await?;
        postro::execute("drop table if exists post", &mut conn).execute().await?;
        postro::execute("create table post(id serial, tag text, name text)", &mut conn).execute().await?;
        task(&mut conn, "dedicated".into()).await?;
    }

    let mut pool = Pool::connect_lazy(&var("DATABASE_URL").unwrap())?;
    let mut handles = vec![];

    doc_example(pool.clone()).await?;

    for i in 0..24 {
        handles.push(tokio::spawn(task(pool.clone(),format!("thread {i}").into())));
    }

    for handle in std::mem::take(&mut handles) {
        handle.await.unwrap()?;
    }

    let foo: Vec<Post> = postro::query("select * from post", &mut pool).fetch_all().await?;

    tracing::info!("{foo:#?}");

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

    postro::execute("CREATE TEMP TABLE foo(id int)", &mut pool)
        .execute()
        .await?;

    let mut handles = vec![];

    for i in 0..14 {
        let mut pool = pool.clone();
        let t = tokio::spawn(async move {
            postro::execute("INSERT INTO foo(id) VALUES($1)", &mut pool)
                .bind(i)
                .execute()
                .await
        });
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

async fn task<E: Executor>(conn: E, id: Cow<'static,str>) -> Result<()> {
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
            .bind(id.as_ref())
            .bind(&format!("NotExists: {id}"))
            .execute()
            .await?;
    }

    let (_post_id,) = postro::query::<_, _, (i32,)>("insert into post(tag,name) values($1,$2) returning id", &mut conn)
        .bind(id.as_ref())
        .bind(&format!("Post from: {id}"))
        .fetch_one()
        .await?;

    let post = postro::query::<_, _, Post>("select * from post", &mut conn)
        .fetch_all()
        .await?;

    assert!(
        post.iter()
            .find(|e| e.tag == id)
            .map(|e| e.name == format!("Post from: {id}"))
            .unwrap()
    );

    postro::execute("insert into post(tag,name) values($1,$2)", &mut conn)
        .bind(id.as_ref())
        .bind(&format!("Exectute for: {id}"))
        .execute()
        .await?;

    let mut stream = postro::query::<_, _, Post>("select * from post", &mut conn).fetch();

    while let Some(post) = stream.try_next().await? {
        let _ = post;
    }

    {
        let mut tx = postro::query::begin(&mut conn).await?;
        postro::execute("insert into post(tag,name) values($1,$2)", &mut tx)
            .bind(id.as_ref())
            .bind(&format!("Transaction from: {id}"))
            .execute()
            .await?;
        tx.commit().await?;
    }

    Ok(())
}
