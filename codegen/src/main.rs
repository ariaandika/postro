use pg_type::Parser;

mod pg_type;

const PG_TYPE: &str = include_str!("../pg_type.dat");

fn main() -> anyhow::Result<()> {
    let result = Parser::new(PG_TYPE)?.parse()?;
    dbg!(result);
    Ok(())
}
