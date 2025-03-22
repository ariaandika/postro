use parser::PgTypeCollector;

mod parser;

const PG_TYPE: &str = include_str!("../pg_type.dat");
#[allow(unused)]
const PG_RANGE: &str = include_str!("../pg_range.dat");

fn main() -> anyhow::Result<()> {
    let pg_type = PgTypeCollector::parser(PG_TYPE)?.parse()?;
    dbg!(pg_type);
    // let pg_range = Parser::new(PG_RANGE)?.parse()?;
    // dbg!(pg_range);
    Ok(())
}
