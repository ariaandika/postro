#![allow(dead_code)]
use postro::{Result, Table};

#[derive(Table)]
struct Postro {
    #[sql(id)]
    id: i32,
    name: String,
    #[sql("now()")]
    created_at: String,
    content: String,
}

#[derive(Table)]
#[sql("foo_bar")]
struct PostroNew {}

pub async fn main() -> Result<()> {
    assert_eq!(Postro::TABLE, "postro");
    assert_eq!(
        Postro::INSERT,
        "INSERT INTO postro(name,created_at,content) VALUES($1,now(),$2)"
    );
    assert_eq!(PostroNew::TABLE, "foo_bar");
    Ok(())
}
