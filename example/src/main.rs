use std::env::var;
use futures::TryStreamExt;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let mut conn = qs::PgConnection::connect(&var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    qs::query("create temp table post(id serial, name text)", &mut conn)
        .execute()
        .await
        .unwrap();

    let (id,) = qs::query("insert into post(name) values($1) returning id", &mut conn)
        .bind("Foo")
        .fetch_one::<(i32,)>()
        .await
        .unwrap();

    let post = qs::query("select * from post", &mut conn)
        .fetch_all::<(i32,String)>()
        .await
        .unwrap();

    assert_eq!(post[0].0, id);

    qs::query("insert into post(name) values($1)", &mut conn)
        .bind("Deez")
        .execute()
        .await
        .unwrap();

    let mut stream = qs::query("select * from post", &mut conn).fetch::<(i32,String)>();

    let p1 = stream.try_next().await.unwrap().unwrap();
    assert_eq!(p1.0, id);
    assert_eq!(p1.1.as_str(), "Foo");

    let p2 = stream.try_next().await.unwrap().unwrap();
    assert_eq!(p2.1.as_str(), "Deez");

    assert!(stream.try_next().await.unwrap().is_none());
}
