use tracing::{Instrument, trace_span};
use tracing_subscriber::{
    EnvFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

use postro::Result;

mod readme;
mod connection;
mod query;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::Registry::default()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    connection::main().instrument(trace_span!("connection")).await?;
    query::main().instrument(trace_span!("query")).await?;

    readme::main().instrument(trace_span!("readme")).await?;

    Ok(())
}

