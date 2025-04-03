use crate::encode::{Encode, Encoded};

pub(crate) const MAX_QUERY_BIND: usize = 16;

pub struct Statement<'a> {
    sql: &'a str,
    args_len: usize,
    args: [Encoded<'a>;MAX_QUERY_BIND],
}

impl<'a> Statement<'a> {
    pub fn new(sql: &'a str) -> Self {
        Self { sql, args_len: 0, args: <_>::default() }
    }

    pub fn bind<E: Encode<'a>>(&mut self, value: E) {
        if self.args_len == MAX_QUERY_BIND {
            panic!("maximum query bind reached")
        }

        self.args[self.args_len] = value.encode();
        self.args_len += 1;
    }

    pub fn sql(&self) -> &str {
        self.sql
    }

    pub fn args(&self) -> &[Encoded<'a>] {
        &self.args[..self.args_len]
    }
}


