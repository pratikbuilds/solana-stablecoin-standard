use anyhow::Result;
use sss_indexer::{IndexerConfig, IndexerService};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("sss_indexer=info".parse()?))
        .with(fmt::layer().json())
        .init();

    let service = IndexerService::new(IndexerConfig::from_env()).await?;
    service.run_live().await
}
