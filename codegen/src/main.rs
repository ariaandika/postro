use anyhow::Context;
use errcode::{ErrCodeGen, ErrCodeParser};
use parser::{PgRangeCollector, PgTypeCollector};
use std::{
    fs::File,
    io::{BufWriter, Write},
};

mod parser;
mod errcode;

const PG_TYPE: &str = include_str!("../pg_type.dat");
const PG_RANGE: &str = include_str!("../pg_range.dat");
const ERRCODES: &str = include_str!("../errcodes.txt");

const ERRCODES_DEST: &str = "src/error/sqlstate.rs";

fn main() -> anyhow::Result<()> {
    let _pg_type = PgTypeCollector::parser(PG_TYPE)?.parse()?;
    let _pg_range = PgRangeCollector::parser(PG_RANGE)?.parse()?;
    let errcodes = ErrCodeParser::new(ERRCODES).parse();

    let mut errcodesrc = BufWriter::new(File::create(ERRCODES_DEST).context(ERRCODES_DEST)?);
    ErrCodeGen::new(errcodes).codegen(&mut errcodesrc)?;
    errcodesrc.flush()?;

    Ok(())
}
