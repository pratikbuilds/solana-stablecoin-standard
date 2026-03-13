use anyhow::{Context, Result};
use chrono::Utc;
use sss_db::Store;
use sss_domain::{
    BlacklistEntryRecord, ChainEvent, ComplianceActionRecord, MintRecord, MintRoleRecord,
    MinterQuotaRecord,
};

use crate::{
    config::IndexerConfig,
    payload::{
        payload_bool, payload_i128, payload_i64, payload_optional_i128, payload_optional_string,
        payload_string,
    },
};

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

    pub async fn ingest_chain_event(&self, event: &ChainEvent) -> Result<()> {
        self.store.insert_chain_event(event).await?;
        self.apply_projection(event).await?;
        Ok(())
    }

    async fn apply_projection(&self, event: &ChainEvent) -> Result<()> {
        match event.event_type.as_str() {
            "StablecoinInitialized" => self.apply_initialized(event).await?,
            "RolesUpdated" | "AuthorityTransferred" => self.apply_roles(event).await?,
            "MinterUpdated" => self.apply_minter_updated(event).await?,
            "TokensMinted" => self.apply_supply_delta(event, true).await?,
            "TokensBurned" => self.apply_supply_delta(event, false).await?,
            "PauseChanged" => self.apply_pause_changed(event).await?,
            "AddressBlacklisted" => self.apply_blacklist(event, true).await?,
            "AddressUnblacklisted" => self.apply_blacklist(event, false).await?,
            "AccountFrozen" | "AccountThawed" | "TokensSeized" => self.apply_compliance_action(event).await?,
            "transfer_rejected_source_blacklisted"
            | "transfer_rejected_destination_blacklisted"
            | "transfer_checked" => self.apply_compliance_action(event).await?,
            _ => {}
        }
        Ok(())
    }

    async fn apply_initialized(&self, event: &ChainEvent) -> Result<()> {
        let mint = MintRecord {
            mint: payload_string(&event.payload, "mint")?,
            preset: payload_string(&event.payload, "preset").unwrap_or_else(|_| "unknown".to_string()),
            authority: payload_string(&event.payload, "authority")?,
            name: payload_string(&event.payload, "name").unwrap_or_default(),
            symbol: payload_string(&event.payload, "symbol").unwrap_or_default(),
            uri: payload_string(&event.payload, "uri").unwrap_or_default(),
            decimals: payload_i64(&event.payload, "decimals").unwrap_or(0) as i16,
            enable_permanent_delegate: payload_bool(&event.payload, "enable_permanent_delegate")
                .unwrap_or(false),
            enable_transfer_hook: payload_bool(&event.payload, "enable_transfer_hook").unwrap_or(false),
            default_account_frozen: payload_bool(&event.payload, "default_account_frozen").unwrap_or(false),
            paused: false,
            total_minted: 0,
            total_burned: 0,
            created_at: event.block_time.unwrap_or_else(Utc::now),
            last_changed_by: payload_string(&event.payload, "authority")?,
            last_changed_at: event.block_time.unwrap_or_else(Utc::now),
            indexed_slot: event.slot,
        };
        self.store.upsert_mint(&mint).await?;
        Ok(())
    }

    async fn apply_roles(&self, event: &ChainEvent) -> Result<()> {
        let mint = payload_string(&event.payload, "mint")?;
        let current = self
            .store
            .get_mint(&mint)
            .await?
            .context("mint must exist before role projection")?;
        let roles = MintRoleRecord {
            mint: mint.clone(),
            master_authority: payload_string(&event.payload, "master_authority")
                .unwrap_or_else(|_| current.authority.clone()),
            pauser: payload_string(&event.payload, "pauser").unwrap_or_else(|_| current.authority.clone()),
            burner: payload_string(&event.payload, "burner").unwrap_or_else(|_| current.authority.clone()),
            blacklister: payload_string(&event.payload, "blacklister").unwrap_or_default(),
            seizer: payload_string(&event.payload, "seizer").unwrap_or_default(),
            updated_at: event.block_time.unwrap_or_else(Utc::now),
            indexed_slot: event.slot,
        };
        self.store.upsert_mint_roles(&roles).await?;
        Ok(())
    }

    async fn apply_minter_updated(&self, event: &ChainEvent) -> Result<()> {
        let quota = MinterQuotaRecord {
            mint: payload_string(&event.payload, "mint")?,
            minter: payload_string(&event.payload, "minter")?,
            quota: payload_i128(&event.payload, "quota")?,
            minted: payload_i128(&event.payload, "minted").unwrap_or(0),
            active: payload_bool(&event.payload, "active").unwrap_or(true),
            updated_at: event.block_time.unwrap_or_else(Utc::now),
            indexed_slot: event.slot,
        };
        self.store.upsert_minter_quota(&quota).await?;
        Ok(())
    }

    async fn apply_supply_delta(&self, event: &ChainEvent, is_mint: bool) -> Result<()> {
        let mint_key = payload_string(&event.payload, "mint")?;
        let amount = payload_i128(&event.payload, "amount")?;
        let mut mint = self
            .store
            .get_mint(&mint_key)
            .await?
            .context("mint must exist before supply updates")?;

        if is_mint {
            mint.total_minted += amount;
        } else {
            mint.total_burned += amount;
        }
        mint.last_changed_by =
            payload_string(&event.payload, "authority").unwrap_or_else(|_| mint.authority.clone());
        mint.last_changed_at = event.block_time.unwrap_or_else(Utc::now);
        mint.indexed_slot = event.slot;
        self.store.upsert_mint(&mint).await?;

        if is_mint {
            let authority =
                payload_string(&event.payload, "authority").unwrap_or_else(|_| mint.authority.clone());
            if let Some(mut quota) = self.store.get_minter_quota(&mint_key, &authority).await? {
                quota.minted += amount;
                quota.updated_at = event.block_time.unwrap_or_else(Utc::now);
                quota.indexed_slot = event.slot;
                self.store.upsert_minter_quota(&quota).await?;
            }
        }

        Ok(())
    }

    async fn apply_pause_changed(&self, event: &ChainEvent) -> Result<()> {
        let mint_key = payload_string(&event.payload, "mint")?;
        let mut mint = self
            .store
            .get_mint(&mint_key)
            .await?
            .context("mint must exist before pause updates")?;
        mint.paused = payload_bool(&event.payload, "paused")?;
        mint.last_changed_by =
            payload_string(&event.payload, "authority").unwrap_or_else(|_| mint.authority.clone());
        mint.last_changed_at = event.block_time.unwrap_or_else(Utc::now);
        mint.indexed_slot = event.slot;
        self.store.upsert_mint(&mint).await?;
        Ok(())
    }

    async fn apply_blacklist(&self, event: &ChainEvent, active: bool) -> Result<()> {
        let entry = BlacklistEntryRecord {
            mint: payload_string(&event.payload, "mint")?,
            wallet: payload_string(&event.payload, "wallet")?,
            reason: payload_string(&event.payload, "reason").unwrap_or_default(),
            blacklisted_by: payload_string(&event.payload, "authority").unwrap_or_default(),
            blacklisted_at: event.block_time.unwrap_or_else(Utc::now),
            active,
            removed_at: if active {
                None
            } else {
                Some(event.block_time.unwrap_or_else(Utc::now))
            },
            indexed_slot: event.slot,
        };
        self.store.upsert_blacklist_entry(&entry).await?;
        self.apply_compliance_action(event).await?;
        Ok(())
    }

    async fn apply_compliance_action(&self, event: &ChainEvent) -> Result<()> {
        let action = ComplianceActionRecord {
            id: None,
            mint: payload_string(&event.payload, "mint")
                .or_else(|_| event.mint.clone().context("missing mint"))?,
            action_type: event.event_type.clone(),
            wallet: payload_optional_string(&event.payload, "wallet"),
            token_account: payload_optional_string(&event.payload, "token_account")
                .or_else(|| payload_optional_string(&event.payload, "account")),
            authority: payload_string(&event.payload, "authority").unwrap_or_default(),
            amount: payload_optional_i128(&event.payload, "amount"),
            tx_signature: event.tx_signature.clone(),
            slot: event.slot,
            related_operation_id: None,
            details: event.payload.clone(),
            occurred_at: event.block_time.unwrap_or_else(Utc::now),
        };
        self.store.insert_compliance_action(&action).await?;
        Ok(())
    }
}
