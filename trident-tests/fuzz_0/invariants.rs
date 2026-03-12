use trident_fuzz::fuzzing::*;

use crate::state_model::{PresetKind, ScenarioState, TokenSlot};

#[derive(Debug, Clone, Copy)]
pub struct ScenarioSnapshot {
    pub total_minted: u64,
    pub total_burned: u64,
    pub supply: u64,
    pub paused: bool,
    pub treasury_balance: u64,
    pub user_a_balance: u64,
    pub user_b_balance: u64,
    pub treasury_frozen: bool,
    pub user_a_frozen: bool,
    pub user_b_frozen: bool,
    pub authority_quota_minted: u64,
    pub minter_quota_minted: u64,
    pub user_a_blacklisted: bool,
    pub user_b_blacklisted: bool,
}

impl ScenarioSnapshot {
    pub fn capture(trident: &mut Trident, scenario: &ScenarioState) -> Self {
        let config = scenario.config(trident);
        let authority_quota = scenario
            .quota(trident, scenario.authority)
            .expect("authority quota should exist");
        let minter_quota = scenario
            .quota(trident, scenario.minter)
            .expect("minter quota should exist");

        Self {
            total_minted: config.total_minted,
            total_burned: config.total_burned,
            supply: scenario.mint_supply(trident),
            paused: config.paused,
            treasury_balance: scenario.token_amount(trident, TokenSlot::Treasury),
            user_a_balance: scenario.token_amount(trident, TokenSlot::UserA),
            user_b_balance: scenario.token_amount(trident, TokenSlot::UserB),
            treasury_frozen: scenario.token_is_frozen(trident, TokenSlot::Treasury),
            user_a_frozen: scenario.token_is_frozen(trident, TokenSlot::UserA),
            user_b_frozen: scenario.token_is_frozen(trident, TokenSlot::UserB),
            authority_quota_minted: authority_quota.minted,
            minter_quota_minted: minter_quota.minted,
            user_a_blacklisted: scenario.blacklist_entry(trident, scenario.user_a).is_some(),
            user_b_blacklisted: scenario.blacklist_entry(trident, scenario.user_b).is_some(),
        }
    }

    pub fn balance_for_slot(&self, slot: TokenSlot) -> u64 {
        match slot {
            TokenSlot::Treasury => self.treasury_balance,
            TokenSlot::UserA => self.user_a_balance,
            TokenSlot::UserB => self.user_b_balance,
        }
    }

    pub fn frozen_for_slot(&self, slot: TokenSlot) -> bool {
        match slot {
            TokenSlot::Treasury => self.treasury_frozen,
            TokenSlot::UserA => self.user_a_frozen,
            TokenSlot::UserB => self.user_b_frozen,
        }
    }
}

pub fn assert_global_invariants(trident: &mut Trident, scenario: &mut ScenarioState) {
    let config = scenario.config(trident);
    let roles = scenario.roles(trident);
    let snapshot = ScenarioSnapshot::capture(trident, scenario);

    assert_eq!(config.mint, scenario.mint, "config mint drifted");
    assert_eq!(roles.mint, scenario.mint, "role config mint drifted");
    assert_eq!(config.authority, scenario.authority, "authority drifted");
    assert_eq!(
        roles.master_authority, scenario.authority,
        "master authority drifted"
    );
    assert!(
        config.total_minted >= scenario.last_total_minted,
        "total minted regressed"
    );
    assert!(
        config.total_burned >= scenario.last_total_burned,
        "total burned regressed"
    );
    assert!(
        config.total_minted >= config.total_burned,
        "burned exceeds minted"
    );
    assert_eq!(
        snapshot.supply,
        config.total_minted - config.total_burned,
        "mint supply diverged from config counters"
    );
    assert_eq!(
        snapshot.treasury_balance + snapshot.user_a_balance + snapshot.user_b_balance,
        snapshot.supply,
        "tracked balances diverged from mint supply"
    );

    match scenario.preset {
        PresetKind::Sss1 => {
            assert!(
                !config.enable_permanent_delegate,
                "SSS-1 should disable delegate"
            );
            assert!(
                !config.enable_transfer_hook,
                "SSS-1 should disable transfer hook"
            );
            assert_eq!(
                roles.blacklister,
                Pubkey::default(),
                "SSS-1 blacklister should be unset"
            );
            assert_eq!(
                roles.seizer,
                Pubkey::default(),
                "SSS-1 seizer should be unset"
            );
            assert!(
                !snapshot.user_a_blacklisted && !snapshot.user_b_blacklisted,
                "SSS-1 should never persist blacklist entries"
            );
        }
        PresetKind::Sss2 => {
            assert!(
                config.enable_permanent_delegate,
                "SSS-2 should enable delegate"
            );
            assert!(
                config.enable_transfer_hook,
                "SSS-2 should enable transfer hook"
            );
            assert_ne!(
                roles.blacklister,
                Pubkey::default(),
                "SSS-2 blacklister should be set"
            );
            assert_ne!(
                roles.seizer,
                Pubkey::default(),
                "SSS-2 seizer should be set"
            );
        }
    }

    scenario.last_total_minted = config.total_minted;
    scenario.last_total_burned = config.total_burned;
}

pub fn assert_snapshot_unchanged(before: &ScenarioSnapshot, after: &ScenarioSnapshot, label: &str) {
    assert_eq!(
        before.total_minted, after.total_minted,
        "{label}: total_minted changed"
    );
    assert_eq!(
        before.total_burned, after.total_burned,
        "{label}: total_burned changed"
    );
    assert_eq!(before.supply, after.supply, "{label}: supply changed");
    assert_eq!(before.paused, after.paused, "{label}: paused changed");
    assert_eq!(
        before.treasury_balance, after.treasury_balance,
        "{label}: treasury_balance changed"
    );
    assert_eq!(
        before.user_a_balance, after.user_a_balance,
        "{label}: user_a_balance changed"
    );
    assert_eq!(
        before.user_b_balance, after.user_b_balance,
        "{label}: user_b_balance changed"
    );
    assert_eq!(
        before.authority_quota_minted, after.authority_quota_minted,
        "{label}: authority quota minted changed"
    );
    assert_eq!(
        before.minter_quota_minted, after.minter_quota_minted,
        "{label}: minter quota minted changed"
    );
    assert_eq!(
        before.user_a_blacklisted, after.user_a_blacklisted,
        "{label}: user_a blacklist changed"
    );
    assert_eq!(
        before.user_b_blacklisted, after.user_b_blacklisted,
        "{label}: user_b blacklist changed"
    );
}
