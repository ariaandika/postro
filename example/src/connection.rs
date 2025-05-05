use std::env::var;
use postro::{query, Connection, Pool, Result};


pub async fn main() -> Result<()> {

    let mut conn = Connection::connect(&var("DATABASE_URL").unwrap()).await?;
    query::<_, _, ()>("SELECT 1", &mut conn).fetch_all().await?;
    conn.close().await?;

    let mut conn = Connection::connect_env().await?;
    query::<_, _, ()>("SELECT 1", &mut conn).fetch_all().await?;
    conn.close().await?;

    let mut pool = Pool::connect(&var("DATABASE_URL").unwrap()).await?;
    query::<_, _, ()>("SELECT 1", &mut pool).fetch_all().await?;
    drop(pool);

    let mut pool = Pool::connect_env().await?;
    query::<_, _, ()>("SELECT 1", &mut pool).fetch_all().await?;
    drop(pool);

    let mut pool = Pool::connect_lazy(&var("DATABASE_URL").unwrap())?;
    query::<_, _, ()>("SELECT 1", &mut pool).fetch_all().await?;
    drop(pool);

    // TODO:
    // let mut pool = Pool::connect_lazy_env()?;
    // query::<_, _, ()>("SELECT 1", &mut pool).fetch_all().await?;
    // drop(pool);

    Ok(())
}

