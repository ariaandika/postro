use super::value::ValueRef;


pub(crate) const MAX_QUERY_BIND: usize = 16;

pub struct Statement<'a> {
    sql: &'a str,
    args_len: usize,
    args: [ValueRef<'a>;MAX_QUERY_BIND],
}

impl<'a> Statement<'a> {
    pub fn new(sql: &'a str) -> Self {
        Self { sql, args_len: 0, args: [const { ValueRef::Null };MAX_QUERY_BIND] }
    }

    pub fn bind(&mut self, value: ValueRef<'a>) {
        if self.args_len == MAX_QUERY_BIND {
            panic!("maximum query bind reached")
        }

        self.args[self.args_len] = value;
        self.args_len += 1;
    }

    pub fn sql(&self) -> &str {
        self.sql
    }

    pub fn args(&self) -> &[ValueRef<'a>] {
        &self.args[..self.args_len]
    }
}


