# Postro Postgres Driver

An Async PostgreSQL Driver and Tools, designed with an API similar to
[sqlx](https://github.com/launchbadge/sqlx), and smaller dependency tree.

## Installation

To install `postro`, run:

```bash
cargo add postro
```

or add this line to your `Cargo.toml` in `[dependencies]` section:

```toml
postro = "0.1.1"
```

## Usage

The general usage is to use database pooling via the `Pool` API:

```rust
use postro::{FromRow, Pool, Result, execute, query};

// automatically extract query result
#[derive(Debug, FromRow)]
struct Post {
    id: i32,
    name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // will read the `DATABASE_URL` environment variable
    let mut pool = Pool::connect_env().await?;
    let mut handles = vec![];

    // execute a statement
    execute("CREATE TABLE post(id serial, name text)", &mut pool).await?;

    for i in 0..24 {
        // cloning pool is cheap and share the same connection pool
        let mut pool = pool.clone();

        handles.push(tokio::spawn(async move {
            execute("INSERT INTO post(name) VALUES($1)", &mut pool)
                .bind(&format!("thread{i}"))
                .await
        }));
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
```

see the documentation for [more details](https://docs.rs/postro)

## License
This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.
