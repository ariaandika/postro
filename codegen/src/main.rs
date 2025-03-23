use anyhow::Context;
use errcode::{ErrCodeGen, ErrCodeParser};
use pg::{PgRangeCollector, PgTypeCollector};
use std::{
    fs::File,
    io::{BufWriter, Write},
};

mod parser;

mod pg;
mod errcode;

const PG_TYPE: &str = include_str!("../pg_type.dat");
const PG_RANGE: &str = include_str!("../pg_range.dat");
const ERRCODES: &str = include_str!("../errcodes.txt");

const PG_DEST: &str = "postgres-types/src/type_gen.rs";
const ERRCODES_DEST: &str = "src/error/sqlstate.rs";

fn main() -> anyhow::Result<()> {
    let types = PgTypeCollector::parser(PG_TYPE)?.parse()?;
    let ranges = PgRangeCollector::parser(PG_RANGE)?.parse()?;
    let errcodes = ErrCodeParser::new(ERRCODES).parse();

    let mut buffer = BufWriter::new(File::create(PG_DEST).context(PG_DEST)?);
    pg::codegen(types, ranges, &mut buffer)?;
    buffer.flush()?;

    let mut buffer = BufWriter::new(File::create(ERRCODES_DEST).context(ERRCODES_DEST)?);
    ErrCodeGen::new(errcodes).codegen(&mut buffer)?;
    buffer.flush()?;

    Ok(())
}
