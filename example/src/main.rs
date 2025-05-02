use futures::TryStreamExt;
use std::{borrow::Cow, env::var};
use tracing::Instrument;

use postro::{Connection, DecodeError, Executor, FromColumn, FromRow, Pool, Result, Row};

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

    let pool = Pool::connect_lazy(&var("DATABASE_URL").unwrap())?;
    let mut handles = vec![];

    for i in 0..24 {
        handles.push(tokio::spawn(task(pool.clone(),format!("thread {i}").into())));
    }

    for handle in std::mem::take(&mut handles) {
        handle.await.unwrap()?;
    }

    let foo: Vec<Post> = postro::query("select * from post", &pool).fetch_all().await?;

    tracing::info!("{foo:#?}");

    Ok(())
}

#[derive(Debug)]
struct Post {
    #[allow(unused)]
    id: i32,
    name: String,
    tag: String,
}

impl FromRow for Post {
    fn from_row(row: Row) -> Result<Self, DecodeError> {
        let mut id = None;
        let mut name = None;
        let mut tag = None;

        for column in row {
            let col = column?;
            match col.name() {
                "id" => id = Some(<_>::decode(col)?),
                "name" => name = Some(<_>::decode(col)?),
                "tag" => tag = Some(<_>::decode(col)?),
                _ => {}
            }
        }

        Ok(Self {
            id: id.ok_or(DecodeError::ColumnNotFound("id".into()))?,
            name: name.ok_or(DecodeError::ColumnNotFound("name".into()))?,
            tag: tag.ok_or(DecodeError::ColumnNotFound("tag".into()))?,
        })
    }
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
