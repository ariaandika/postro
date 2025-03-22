use std::io::BufWriter;

use errcode::{ErrCodeGen, ErrCodeParser};
use parser::{PgRangeCollector, PgTypeCollector};

mod parser;
mod errcode;

const PG_TYPE: &str = include_str!("../pg_type.dat");
const PG_RANGE: &str = include_str!("../pg_range.dat");
const ERRCODES: &str = include_str!("../errcodes.txt");

fn main() -> anyhow::Result<()> {
    let _pg_type = PgTypeCollector::parser(PG_TYPE)?.parse()?;
    let _pg_range = PgRangeCollector::parser(PG_RANGE)?.parse()?;
    let errcodes = ErrCodeParser::new(ERRCODES).parse();

    let mut errcodesrc = Vec::new();
    ErrCodeGen::new(errcodes).codegen(&mut BufWriter::new(&mut errcodesrc))?;

    println!("{}", String::from_utf8(errcodesrc).unwrap());

    Ok(())
}
