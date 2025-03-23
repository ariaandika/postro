use crate::parser::{Collector, Map, Parser};
use anyhow::{bail, Context, Result};
use regex::Regex;
use std::collections::HashMap;

// NOTE: PG_TYPE

#[derive(Debug)]
#[allow(dead_code)]
pub struct Type {
    pub oid: u32,
    pub typname: String,
    pub typcategory: String,
    pub map: HashMap<String,String>,
}

impl std::ops::Index<&str> for Type {
    type Output = String;

    fn index(&self, index: &str) -> &Self::Output {
        std::ops::Index::index(&self.map, index)
    }
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
        let typname = map
            .remove("typname")
            .context("missing `typname`")?;
        let typcategory = map
            .remove("typcategory")
            .context("missing `typcategory`")?;
        self.types.push(Type { oid, typname, typcategory, map, });
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

// NOTE: codegen

struct TypeGen {
    name: String,
    variant: String,
    ident: String,
    kind: String,
    typtype: Option<String>,
    element: u32,
    doc: String,
}

pub fn codegen<W: std::io::Write>(raw_types: Vec<Type>, raw_ranges: Vec<Range>, writer: &mut W) -> Result<()> {
    let oids_by_name = raw_types
        .iter()
        .map(|m|(m.typname.clone(), m.oid))
        .collect::<HashMap<_, _>>();

    let range_elements = raw_ranges
        .iter()
        .map(|m|(oids_by_name[&m.rngtypid],oids_by_name[&m.rngsubtype]))
        .collect::<HashMap<_, _>>();

    let multi_range_elements = raw_ranges
        .iter()
        .map(|m|(oids_by_name[&m.rngmultitypid],oids_by_name[&m.rngsubtype]))
        .collect::<HashMap<_, _>>();

    let range_vector_re = Regex::new("(range|vector)$").unwrap();
    let array_re = Regex::new("^_(.*)").unwrap();

    let mut types = std::collections::BTreeMap::new();

    for Type { oid, typname: name, typcategory: kind, map } in raw_types {

        // we need to be able to pull composite fields and enum variants at runtime
        if matches!(&*kind,"C"|"E") {
            continue;
        }

        let typtype = map.get("typtype").cloned();

        let ident = range_vector_re.replace(&name, "_$1");
        let ident = array_re.replace(&ident, "${1}_array");
        let variant = snake_to_camel(&ident);
        let ident = ident.to_ascii_uppercase();

        let element = match &*kind {
            "R" => match typtype
                .as_ref()
                .expect("range type must have typtype")
                .as_str()
            {
                "r" => range_elements[&oid],
                "m" => multi_range_elements[&oid],
                typtype => panic!("invalid range typtype {}", typtype),
            },
            "A" => oids_by_name[&map["typelem"]],
            _ => 0,
        };

        let doc_name = array_re.replace(&name, "$1[]").to_ascii_uppercase();
        let mut doc = doc_name.clone();
        if let Some(descr) = map.get("descr") {
            std::fmt::Write::write_fmt(&mut doc, format_args!(" - {descr}"))?;
        }
        // let doc = Escape::new(doc.as_bytes().iter().cloned()).collect();
        // let doc = String::from_utf8(doc).unwrap();
        let doc = doc;

        if let Some(array_type_oid) = map.get("array_type_oid") {
            let array_type_oid = array_type_oid.parse::<u32>().unwrap();

            let name = format!("_{}", name);
            let variant = format!("{}Array", variant);
            let doc = format!("{}&#91;&#93;", doc_name);
            let ident = format!("{}_ARRAY", ident);

            let type_gen = TypeGen {
                name,
                variant,
                ident,
                kind: "A".to_string(),
                typtype: None,
                element: oid,
                doc,
            };
            types.insert(array_type_oid, type_gen);
        }

        let type_gen = TypeGen {
            name,
            variant,
            ident,
            kind,
            typtype,
            element,
            doc,
        };
        types.insert(oid, type_gen);
    }


    writer.write_all(b"\
//! Autogenerated file from postgres 'pg_range.dat' and 'pg_range.dat'
use std::sync::Arc;

use crate::{Kind, Oid, Type};

#[derive(PartialEq, Eq, Debug, Hash)]
pub struct Other {
    pub name: String,
    pub oid: Oid,
    pub kind: Kind,
    pub schema: String,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum Inner {
")?;

    for TypeGen { variant, .. } in types.values() {
        writeln!(writer, "    {variant},")?;
    }

    writer.write_all(b"
    Other(Arc<Other>),
}

impl Inner {
    pub fn from_oid(oid: Oid) -> Option<Inner> {
        match oid {
")?;

    for (oid, TypeGen { variant, .. }) in &types {
        writeln!(
            writer,
"            {oid} => Some(Inner::{variant}),"
        )?;
    }

    writer.write_all(
b"            _ => None,
        }
    }

    pub fn oid(&self) -> Oid {
        match *self {
")?;

    for (oid, TypeGen { variant, .. }) in &types {
        writeln!(
            writer,
"            Inner::{variant} => {oid},"
        )?;
    }

    writer.write_all(
b"            Inner::Other(ref u) => u.oid,
        }
    }

    pub fn kind(&self) -> &Kind {
        match *self {
")?;

    for t in types.values() {
        let kind = match &*t.kind {
            "P" => "Pseudo".to_owned(),
            "A" => format!("Array(Type(Inner::{}))",types[&t.element].variant),
            "R" => match t.typtype.as_ref().expect("convention").as_str()
            {
                "r" => format!("Range(Type(Inner::{}))", types[&t.element].variant),
                "m" => format!("Multirange(Type(Inner::{}))", types[&t.element].variant),
                typtype => panic!("invalid range typtype {}", typtype),
            }
            _ => "Simple".to_owned(),
        };

        writeln!(
            writer,
"            Inner::{} => &Kind::{},",
            t.variant, kind,
        )?;
    }

    writer.write_all(
b"            Inner::Other(ref u) => &u.kind,
        }
    }

    pub fn name(&self) -> &str {
        match *self {
")?;

    for TypeGen { variant, name, .. } in types.values() {
        writeln!(
            writer,
"            Inner::{variant} => \"{name}\","
        )
        .unwrap();
    }

    writer.write_all(
b"            Inner::Other(ref u) => &u.name,
        }
    }
}

impl Type {
")?;

    for TypeGen { doc, ident, variant, .. } in types.values() {
        writeln!(
            writer,
"    /// {doc}
    pub const {ident}: Type = Type(Inner::{variant});
",
        )?;
    }

    writer.write_all(b"}")?;

    Ok(())
}

fn snake_to_camel(s: &str) -> String {
    let mut out = String::new();

    let mut upper = true;
    for ch in s.chars() {
        if ch == '_' {
            upper = true;
        } else {
            let ch = if upper {
                upper = false;
                ch.to_ascii_uppercase()
            } else {
                ch
            };
            out.push(ch);
        }
    }

    out
}

