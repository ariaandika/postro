use std::hash::{DefaultHasher, Hash, Hasher};

use crate::{
    Result,
    common::Stack,
    encode::{Encode, Encoded},
    ext::UsizeExt,
    postgres::{PgFormat, ProtocolError, backend, frontend},
    row::{self, FromRow, Row},
    statement::{PortalName, StatementName},
    transport::PgTransport,
};

pub fn query<'val, IO: PgTransport>(sql: &str, io: IO) -> Query<'_, 'val, IO> {
    Query { sql, io, params: Stack::with_size(), persistent: true }
}

pub struct Query<'sql, 'val, IO> {
    sql: &'sql str,
    io: IO,
    params: Stack<Encoded<'val>>,
    persistent: bool,
}

impl<'val, IO> Query<'_, 'val, IO> {
    /// Disable persistent prepared statement.
    ///
    /// This will use unnamed prepared statement under the hood,
    /// which optimized for the case of executing a query only once and then discarding it.
    ///
    /// <https://www.postgresql.org/docs/current/protocol-overview.html#PROTOCOL-QUERY-CONCEPTS>
    pub fn once(mut self) -> Self {
        self.persistent = false;
        self
    }

    pub fn bind<V: Encode<'val>>(mut self, value: V) -> Self {
        self.params.push(value.encode());
        self
    }
}

impl<IO> Query<'_, '_, IO>
where
    IO: PgTransport,
{
    pub async fn fetch_all<R: FromRow>(mut self) -> Result<Vec<R>> {
        let sqlid = {
            let mut buf = DefaultHasher::new();
            self.sql.hash(&mut buf);
            buf.finish()
        };

        let stmt = match !self.persistent {
            true => StatementName::unnamed(),
            false => match self.io.get_stmt(sqlid) {
                Some(ok) => ok,
                None => {
                    let stmt = StatementName::next();

                    self.io.send(frontend::Parse {
                        prepare_name: stmt.as_str(),
                        sql: self.sql,
                        oids_len: self.params.len() as _,
                        oids: self.params.iter().map(crate::encode::Encoded::oid),
                    });
                    self.io.send(frontend::Flush);
                    self.io.flush().await?;
                    self.io.recv::<backend::ParseComplete>().await?;
                    stmt
                }
            },
        };

        let portal = PortalName::unnamed();

        self.io.send(frontend::Bind {
            portal_name: portal.as_str(),
            stmt_name: stmt.as_str(),
            param_formats_len: 1,
            param_formats: [PgFormat::Binary],
            params_len: self.params.len().to_u16(),
            params_size_hint: self.params.iter().fold(0, |acc,n|{
                acc + 4 + n.value().len().to_u32()
            }),
            params: self.params.into_iter(),
            result_formats_len: 1,
            result_formats: [PgFormat::Binary],
        });
        self.io.send(frontend::Describe {
            kind: b'P',
            name: portal.as_str(),
        });
        self.io.send(frontend::Execute {
            portal_name: portal.as_str(),
            max_row: 0,
        });
        self.io.send(frontend::Flush);
        self.io.flush().await?;

        self.io.recv::<backend::BindComplete>().await?;

        let desc = self.io.recv::<backend::RowDescription>().await?;
        let mut cols = row::decode_row_desc(desc);
        let mut rows = vec![];

        loop {
            use backend::BackendMessage::*;
            match self.io.recv().await? {
                CommandComplete(_) => break,
                DataRow(dr) => rows.push(R::from_row(Row::new(&mut cols, dr))?),
                f => Err(ProtocolError::unexpected_phase(f.msgtype(), "extended query"))?,
            }
        }

        let should_close = match (self.persistent,!stmt.is_unnamed()) {
            (true, true) => !self.io.add_stmt(sqlid, stmt.clone()),
            (_, is_named) => is_named,
        };

        if should_close {
            self.io.send(frontend::Close {
                variant: b'S',
                name: stmt.as_str(),
            });
        }

        self.io.send(frontend::Sync);
        self.io.flush().await?;

        loop {
            use backend::BackendMessage::*;
            match self.io.recv().await? {
                CloseComplete(_) => {}
                ReadyForQuery(_) => break,
                f => Err(ProtocolError::unexpected_phase(f.msgtype(), "extended query"))?,
            }
        }

        Ok(rows)
    }
}

#[cfg(all(test, feature = "tokio"))]
mod test {
    use crate::{stream::PgStream, PgOptions};

    #[test]
    fn test_query() {
        // use crate::{value::ValueRef, types::AsPgType};

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let opt = PgOptions::parse("postgres://cookiejar:cookie@127.0.0.1:5432/postgres").unwrap();
                let mut conn = PgStream::connect(
                    &opt,
                )
                .await
                .unwrap();

                crate::protocol::startup(&opt, &mut conn).await.unwrap();

                let mut rows = super::query("select null,4,$1", &mut conn)
                    .bind("Deez")
                    .fetch_all::<()>()
                    .await
                    .unwrap();

                // dbg!(rows.get_mut(0).unwrap().collect::<Vec<_>>());
            })
    }
}

