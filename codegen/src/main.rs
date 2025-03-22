use parser::{PgRangeCollector, PgTypeCollector};

mod parser;

const PG_TYPE: &str = include_str!("../pg_type.dat");
const PG_RANGE: &str = include_str!("../pg_range.dat");

fn main() -> anyhow::Result<()> {
    let pg_type = PgTypeCollector::parser(PG_TYPE)?.parse()?;
    let pg_range = PgRangeCollector::parser(PG_RANGE)?.parse()?;

    dbg!(pg_type);
    dbg!(pg_range);
    Ok(())
}
