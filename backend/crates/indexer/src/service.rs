use anyhow::Result;
use sss_db::Store;
use sss_domain::InsertEvent;

use crate::config::IndexerConfig;

#[derive(Clone)]
pub struct IndexerService {
    pub store: Store,
    pub config: IndexerConfig,
}

impl IndexerService {
    pub async fn new(config: IndexerConfig) -> Result<Self> {
        let store = Store::connect(&config.database_url).await?;
        store.migrate().await?;
        Ok(Self { store, config })
    }

    pub async fn run_live(&self) -> Result<()> {
        crate::pipeline::run_live(self).await
    }

    pub async fn ingest_event(&self, event: &InsertEvent) -> Result<()> {
        self.store.insert_event(event).await?;
        Ok(())
    }
}
