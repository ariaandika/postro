use crate::parser::{Collector, Map, Parser};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;

// NOTE: PG_TYPE

#[derive(Debug)]
#[allow(dead_code)]
pub struct Type {
    pub oid: u32,
    pub map: HashMap<String,String>,
}

#[derive(Debug)]
pub struct PgTypeCollector {
    pub types: Vec<Type>,
}

impl PgTypeCollector {
    pub fn parser(source: &str) -> Result<Parser<'_, Self>> {
        Parser::new(source, Self { types: <_>::default() })
    }
}

impl Collector for PgTypeCollector {
    type Output = Vec<Type>;

    fn add_map(&mut self, mut map: Map) -> Result<()> {
        let oid = map
            .remove("oid")
            .context("missing `oid`")?
            .parse()
            .context("oid not an integer")?;
        self.types.push(Type { oid, map, });
        Ok(())
    }

    fn finish(self) -> Self::Output {
        self.types
    }
}


// NOTE: PG_RANGE

#[derive(Debug)]
#[allow(dead_code)]
pub struct Range {
    pub rngtypid: String,
    pub rngsubtype: String,
    pub rngmultitypid: String,
    pub rngsubopc: String,
    pub rngcanonical: String,
    pub rngsubdiff: String,
}

#[derive(Debug)]
pub struct PgRangeCollector {
    pub ranges: Vec<Range>,
}

impl PgRangeCollector {
    pub fn parser(source: &str) -> Result<Parser<'_, Self>> {
        Parser::new(source, Self { ranges: <_>::default() })
    }
}

impl Collector for PgRangeCollector {
    type Output = Vec<Range>;

    fn add_map(&mut self, mut map: Map) -> Result<()> {
        let range = Range {
            rngtypid: map.remove("rngtypid").context("missing `rngtypid`")?,
            rngsubtype: map.remove("rngsubtype").context("missing `rngsubtype`")?,
            rngmultitypid: map.remove("rngmultitypid").context("missing `rngmultitypid`")?,
            rngsubopc: map.remove("rngsubopc").context("missing `rngsubopc`")?,
            rngcanonical: map.remove("rngcanonical").context("missing `rngcanonical`")?,
            rngsubdiff: map.remove("rngsubdiff").context("missing `rngsubdiff`")?,
        };
        if let Some((k,v)) = map.into_iter().next() {
            bail!("unexpected `{k}: {v}`");
        }
        self.ranges.push(range);
        Ok(())
    }

    fn finish(self) -> Self::Output {
        self.ranges
    }
}

