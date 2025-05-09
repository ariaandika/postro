#![allow(unused)]
use postro::{query, query_as, Connection, FromRow, Result};

#[derive(FromRow)]
struct Postro {
    id: i32,
    name: String
}

#[derive(FromRow)]
struct PostroTuple(i32,String);

pub async fn main() -> Result<()> {
    let mut conn = Connection::connect_env().await?;

    // Execute

    query("CREATE TEMP TABLE postro(id serial, name text)", &mut conn).await?;

    let row = query("INSERT INTO postro(name) VALUES($1)", &mut conn)
        .bind("Deez")
        .await?;

    query("INSERT INTO postro(name) VALUES('Foo')", &mut conn).await?;

    assert_eq!(row.rows_affected, 1);

    // Queries

    let datas = query_as::<_, _, Postro>("SELECT * FROM postro", &mut conn)
        .fetch_all()
        .await?;

    let datas = query_as::<_, _, PostroTuple>("SELECT * FROM postro", &mut conn)
        .fetch_all()
        .await?;

    Ok(())
}
