use std::sync::atomic::{AtomicU16, Ordering};

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



static STMT_NAME: AtomicU16 = AtomicU16::new(0);

#[derive(Debug, Clone)]
pub struct StatementName([u8;6]);

impl StatementName {
    pub fn unnamed() -> StatementName {
        StatementName([0,0,0,0,0,0])
    }

    pub fn next() -> StatementName {
        let mut buf = [95u8;6];
        let id = STMT_NAME.fetch_add(1, Ordering::SeqCst);
        let mut b = itoa::Buffer::new();
        let id = b.format(id);

        buf[..3].copy_from_slice(b"_qs");

        let i = &id.as_bytes().get(..3).unwrap_or(id.as_bytes());
        buf[3..3 + i.len()].copy_from_slice(i);

        std::str::from_utf8(&buf[..]).expect("itoa's fault");
        StatementName(buf)
    }

    pub fn is_unnamed(&self) -> bool {
        self.0[0] == 0
    }

    pub fn as_str(&self) -> &str {
        if self.is_unnamed() {
            return "";
        }
        // SAFETY: check on construction and is immutable
        unsafe { std::str::from_utf8_unchecked(&self.0[..]) }
    }
}

pub fn next_stmt_name() -> StatementName {
    StatementName::next()
}


static PORTAL_NAME: AtomicU16 = AtomicU16::new(0);

#[derive(Debug)]
pub struct PortalName([u8;6]);

impl PortalName {
    pub fn as_str(&self) -> &str {
        // SAFETY: check on construction and is immutable
        unsafe { std::str::from_utf8_unchecked(&self.0[..]) }
    }
}

pub fn next_portal_name() -> PortalName {
    let mut buf = [0u8;6];
    let id = PORTAL_NAME.fetch_add(1, Ordering::SeqCst);
    let mut b = itoa::Buffer::new();
    let id = b.format(id);

    buf[..3].copy_from_slice(b"_qs");

    let i = &id.as_bytes().get(..3).unwrap_or(id.as_bytes());
    buf[3..3 + i.len()].copy_from_slice(i);

    std::str::from_utf8(&buf[..]).expect("itoa's fault");
    PortalName(buf)
}

