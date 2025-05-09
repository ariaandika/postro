use futures::StreamExt;
use postro::{Connection, Result, begin, query, query_as, query_scalar};

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

    let datas = query_as::<_, _, (i32, String)>("SELECT * FROM postro", &mut conn)
        .fetch_all()
        .await?;

    assert_eq!(datas.len(), 2);

    let (_id, name) = query_as::<_, _, (i32, String)>("SELECT * FROM postro LIMIT 1", &mut conn)
        .fetch_one()
        .await?;

    assert_eq!(name.as_str(), "Deez");
    assert_eq!(name, datas[0].1);

    let data = query_as::<_, _, (i32, String)>("SELECT * FROM postro", &mut conn)
        .fetch_optional()
        .await?;

    assert!(data.is_some());

    let data = query_as::<_, _, (i32, String)>("SELECT * FROM postro LIMIT 0", &mut conn)
        .fetch_optional()
        .await?;

    assert!(data.is_none());

    let mut stream = query_as::<_, _, (i32, String)>("SELECT * FROM postro", &mut conn).fetch();

    while let Some(row) = stream.next().await {
        let (_id, _name) = row?;
    }

    let datas = query("SELECT * FROM postro", &mut conn).fetch_all().await?;

    assert_eq!(
        datas[0].try_get::<_, String>("name").unwrap().as_str(),
        "Deez"
    );

    let datas = query_scalar::<_, _, String>("SELECT name FROM postro", &mut conn)
        .fetch_all()
        .await?;

    assert_eq!(datas[0].as_str(), "Deez");

    let mut tx = begin(&mut conn).await?;
    query("INSERT INTO postro(name) VALUES('Foo')", &mut tx).await?;
    tx.commit().await?;

    // Error case

    query("", &mut conn).await.unwrap_err();
    query("SELECT foo", &mut conn).await.unwrap_err();

    let _err = query_as::<_, _, (i32, String)>("SELECT * FROM postro LIMIT 0", &mut conn)
        .fetch_one()
        .await
        .unwrap_err();

    Ok(())
}

