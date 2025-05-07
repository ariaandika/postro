use tracing::{Instrument, trace_span};
use tracing_subscriber::{
    EnvFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

use postro::Result;

mod connection;
mod query;
mod from_row;
mod table;
mod error;

mod readme;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::Registry::default()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    connection::main().instrument(trace_span!("connection")).await?;
    query::main().instrument(trace_span!("query")).await?;
    from_row::main().await?;
    table::main().await?;
    error::main().await?;

    readme::main().instrument(trace_span!("readme")).await?;

    Ok(())
}

