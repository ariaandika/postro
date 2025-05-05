use postro::{FromRow, Pool, Result, execute, query};
use tracing::{Instrument, trace_span};

// automatically extract query result
#[derive(Debug, FromRow)]
struct Post {
    #[allow(unused)]
    id: i32,
    name: String,
}

pub async fn main() -> Result<()> {
    // will read the `DATABASE_URL` environment variable
    let mut pool = Pool::connect_env().await?;
    let mut handles = vec![];

    execute("DROP TABLE IF EXISTS post", &mut pool).await?;

    // execute a statement
    execute("CREATE TABLE post(id serial, name text)", &mut pool).await?;

    for id in 0..24 {
        // cloning pool is cheap and share the same connection pool
        let mut pool = pool.clone();

        handles.push(tokio::spawn(async move {
            execute("INSERT INTO post(name) VALUES($1)", &mut pool)
                .bind(&format!("thread{id}"))
                .await
        }.instrument(trace_span!("thread",id))));
    }

    for h in handles {
        h.await.unwrap()?;
    }

    // extract query result
    let posts = query::<_, _, Post>("SELECT * FROM post", &mut pool)
        .fetch_all()
        .await?;

    assert!(posts.iter().any(|e| e.name.as_str() == "thread23"));
    assert_eq!(posts.len(), 24);

    Ok(())
}
