use futures::StreamExt;
use postro::{execute, query, Connection, Result};


pub async fn main() -> Result<()> {
    let mut conn = Connection::connect_env().await?;

    execute("CREATE TEMP TABLE postro(id serial, name text)", &mut conn)
        .execute()
        .await?;

    // TODO:
    // execute("CREATE TEMP TABLE postro(id serial, name text)", &mut conn).await?;
    // query("CREATE TEMP TABLE postro(id serial, name text)", &mut conn).await?;

    let row = execute("INSERT INTO postro(name) VALUES($1)", &mut conn)
        .bind("Deez")
        .execute()
        .await?;

    assert_eq!(row,1);



    let datas = query::<_, _, (i32,String)>("SELECT * FROM postro", &mut conn)
        .fetch_all()
        .await?;

    assert_eq!(datas.len(), 1);

    let (_id,name) = query::<_, _, (i32,String)>("SELECT * FROM postro LIMIT 1", &mut conn)
        .fetch_one()
        .await?;

    assert_eq!(name.as_str(), "Deez");

    // TODO:
    // let data = query::<_, _, (i32,String)>("SELECT * FROM postro", &mut conn)
    //     .fetch_optional()
    //     .await?;



    let mut stream = query::<_, _, (i32,String)>("SELECT * FROM postro", &mut conn).fetch();

    while let Some(row) = stream.next().await {
        let (_id,_name) = row?;
    }

    // TODO:
    // let datas = query_row("SELECT * FROM postro", &mut conn)
    //     .fetch_all()
    //     .await?;

    Ok(())
}

