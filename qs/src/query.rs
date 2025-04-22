use crate::{
    encode::{Encode, Encoded},
    row::FromRow,
    transport::PgTransport,
};

mod fetch_all;
mod fetch;

pub use fetch_all::FetchAll;
pub use fetch::Fetch;

pub fn query<'val, IO: PgTransport>(sql: &str, io: IO) -> Query<'_, 'val, IO> {
    Query { sql, io, params: Vec::new(), persistent: true }
}

pub struct Query<'sql, 'val, IO> {
    sql: &'sql str,
    io: IO,
    params: Vec<Encoded<'val>>,
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

impl<'sql, 'val, IO> Query<'sql, 'val, IO>
where
    IO: PgTransport,
{
    pub fn fetch_all_v2<R: FromRow>(self) -> FetchAll<'sql, 'val, R, IO> {
        FetchAll::new(self.sql, self.io, self.params, self.persistent)
    }

    pub fn fetch<R: FromRow>(self) -> Fetch<'sql, 'val, R, IO> {
        Fetch::new(self.sql, self.io, self.params, self.persistent)
    }
}

