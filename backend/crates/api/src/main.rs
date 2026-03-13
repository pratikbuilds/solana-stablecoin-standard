use anyhow::Result;
use sss_api::{run, ApiConfig};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("sss_api=info".parse()?))
        .with(fmt::layer().json())
        .init();

    run(ApiConfig::from_env()).await
}
