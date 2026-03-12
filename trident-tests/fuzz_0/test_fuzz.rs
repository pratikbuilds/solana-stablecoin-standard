use anchor_lang::error::ERROR_CODE_OFFSET;
use fuzz_accounts::*;
use invariants::{assert_global_invariants, assert_snapshot_unchanged, ScenarioSnapshot};
use stablecoin::errors::StablecoinError;
use transfer_hook::TransferHookError;
use trident_fuzz::fuzzing::*;
mod fuzz_accounts;
mod invariants;
mod state_model;
mod types;
use state_model::{
    add_to_blacklist_ix, burn_ix, freeze_account_ix, mint_ix, pause_ix, remove_from_blacklist_ix,
    seize_ix_with_amount, thaw_account_ix, transfer_checked_with_hook_ix, unpause_ix,
    update_minter_ix, ScenarioState, TokenSlot,
};

macro_rules! process_ix {
    ($trident:expr, $ix:expr, $label:expr) => {
        $trident.process_transaction(&[$ix], Some($label))
    };
}

macro_rules! assert_custom_error {
    ($result:expr, $error:expr) => {{
        let expected = ERROR_CODE_OFFSET + ($error as u32);
        assert!(
            $result.is_custom_error_with_code(expected),
            "expected custom error {expected}, got {:?}\nlogs: {}",
            $result.get_custom_error_code(),
            $result.logs()
        );
    }};
}

#[derive(FuzzTestMethods)]
struct FuzzTest {
    /// Trident client for interacting with the Solana program
    trident: Trident,
    /// Storage for all account addresses used in fuzz testing
    fuzz_accounts: AccountAddresses,
    /// Stateful model used to drive multi-step fuzz flows.
    scenario: Option<ScenarioState>,
}

#[flow_executor]
impl FuzzTest {
    fn new() -> Self {
        Self {
            trident: Trident::default(),
            fuzz_accounts: AccountAddresses::default(),
            scenario: None,
        }
    }

    #[init]
    fn start(&mut self) {
        let mut scenario = ScenarioState::initialize(&mut self.trident);
        Self::debug_log(format!(
            "initialized preset={:?} mint={} authority={} minter={}",
            scenario.preset, scenario.mint, scenario.authority, scenario.minter
        ));
        assert_global_invariants(&mut self.trident, &mut scenario);
        self.scenario = Some(scenario);
    }

    #[flow]
    fn lifecycle_flow(&mut self) {
        match self.trident.random_from_range(0..7) {
            0 => self.lifecycle_update_minter(),
            1 => self.lifecycle_mint(),
            2 => self.lifecycle_pause_toggle(),
            3 => self.lifecycle_freeze_toggle(),
            4 => self.lifecycle_burn(),
            5 => self.lifecycle_blacklist_or_seize(),
            _ => self.lifecycle_transfer_hook(),
        }

        let scenario = self.scenario.as_mut().expect("scenario initialized");
        assert_global_invariants(&mut self.trident, scenario);
    }

    #[flow]
    fn negative_flow(&mut self) {
        match self.trident.random_from_range(0..8) {
            0 => self.negative_unauthorized_pause(),
            1 => self.negative_unauthorized_update_minter(),
            2 => self.negative_mint_while_paused(),
            3 => self.negative_burn_while_paused(),
            4 => self.negative_zero_amount_mint(),
            5 => self.negative_blacklist_or_seize(),
            6 => self.negative_invalid_treasury_seize(),
            _ => self.negative_transfer_hook_blacklist(),
        }

        let scenario = self.scenario.as_mut().expect("scenario initialized");
        assert_global_invariants(&mut self.trident, scenario);
    }

    #[end]
    fn end(&mut self) {
        if let Some(mut scenario) = self.scenario.take() {
            Self::debug_log(format!(
                "completed preset={:?} total_minted={} total_burned={}",
                scenario.preset, scenario.last_total_minted, scenario.last_total_burned
            ));
            assert_global_invariants(&mut self.trident, &mut scenario);
        }
    }
}

fn main() {
    FuzzTest::fuzz(1000, 100);
}

impl FuzzTest {
    fn debug_enabled() -> bool {
        std::env::var("TRIDENT_FUZZ_DEBUG").is_ok()
    }

    fn debug_log(message: impl AsRef<str>) {
        if Self::debug_enabled() {
            println!("[fuzz_0] {}", message.as_ref());
        }
    }

    fn scenario(&self) -> ScenarioState {
        *self.scenario.as_ref().expect("scenario initialized")
    }

    fn lifecycle_update_minter(&mut self) {
        Self::debug_log("flow=lifecycle_update_minter");
        let scenario = self.scenario();
        let quota = self.trident.random_from_range(1_000_u64..25_000_u64);
        let active = self.trident.random_bool();
        let result = process_ix!(
            &mut self.trident,
            update_minter_ix(
                scenario.authority,
                scenario.mint,
                scenario.minter,
                quota,
                active
            ),
            "UpdateMinter"
        );
        assert!(
            result.is_success(),
            "update_minter should succeed: {}",
            result.logs()
        );

        let after = scenario
            .quota(&mut self.trident, scenario.minter)
            .expect("updated minter quota should exist");
        assert_eq!(after.quota, quota, "quota not updated");
        assert_eq!(after.active, active, "active flag not updated");
    }

    fn lifecycle_mint(&mut self) {
        Self::debug_log("flow=lifecycle_mint");
        let scenario = self.scenario();
        if scenario.config(&mut self.trident).paused {
            let unpause = process_ix!(
                &mut self.trident,
                unpause_ix(scenario.authority, scenario.mint),
                "UnpauseBeforeMint"
            );
            assert!(
                unpause.is_success(),
                "unpause before mint should succeed: {}",
                unpause.logs()
            );
        }
        let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);

        let (authority, quota_before) = {
            let authority_quota = scenario
                .quota(&mut self.trident, scenario.authority)
                .expect("authority quota should exist");
            let minter_quota = scenario
                .quota(&mut self.trident, scenario.minter)
                .expect("minter quota should exist");
            if minter_quota.active
                && minter_quota.minted < minter_quota.quota
                && self.trident.random_bool()
            {
                (scenario.minter, minter_quota)
            } else {
                (scenario.authority, authority_quota)
            }
        };

        let remaining = quota_before.quota.saturating_sub(quota_before.minted);
        if remaining == 0 || !quota_before.active {
            let result = process_ix!(
                &mut self.trident,
                update_minter_ix(
                    scenario.authority,
                    scenario.mint,
                    authority,
                    quota_before.quota.saturating_add(10_000),
                    true,
                ),
                "RepairMinterQuota"
            );
            assert!(
                result.is_success(),
                "repair quota should succeed: {}",
                result.logs()
            );
        }

        let slot = scenario.random_slot(&mut self.trident);
        if slot != TokenSlot::Treasury && scenario.token_is_frozen(&mut self.trident, slot) {
            let thaw = process_ix!(
                &mut self.trident,
                thaw_account_ix(scenario.authority, scenario.mint, scenario.tracked_account(slot)),
                "ThawBeforeMint"
            );
            assert!(
                thaw.is_success(),
                "thaw before mint should succeed: {}",
                thaw.logs()
            );
        }
        let refreshed_quota = scenario
            .quota(&mut self.trident, authority)
            .expect("quota should exist");
        let amount = self.trident.random_from_range(
            1_u64
                ..=refreshed_quota
                    .quota
                    .saturating_sub(refreshed_quota.minted)
                    .max(1)
                    .min(500),
        );

        let result = process_ix!(
            &mut self.trident,
            mint_ix(
                authority,
                scenario.mint,
                scenario.tracked_account(slot),
                amount
            ),
            "Mint"
        );
        assert!(
            result.is_success(),
            "mint should succeed: {}",
            result.logs()
        );

        let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        assert_eq!(
            after.total_minted,
            before.total_minted + amount,
            "mint counter drifted"
        );
        assert_eq!(
            after.total_burned, before.total_burned,
            "burn counter changed during mint"
        );
        assert_eq!(
            after.supply,
            before.supply + amount,
            "supply should increase after mint"
        );
        assert_eq!(
            after.balance_for_slot(slot),
            before.balance_for_slot(slot) + amount,
            "destination balance should increase after mint"
        );
        if authority == scenario.authority {
            assert_eq!(
                after.authority_quota_minted,
                before.authority_quota_minted + amount,
                "authority quota minted drifted"
            );
        } else {
            assert_eq!(
                after.minter_quota_minted,
                before.minter_quota_minted + amount,
                "minter quota minted drifted"
            );
        }
    }

    fn lifecycle_pause_toggle(&mut self) {
        let scenario = self.scenario();
        let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        let ix = if before.paused {
            unpause_ix(scenario.authority, scenario.mint)
        } else {
            pause_ix(scenario.authority, scenario.mint)
        };
        let result = process_ix!(&mut self.trident, ix, "PauseToggle");
        assert!(
            result.is_success(),
            "pause toggle should succeed: {}",
            result.logs()
        );

        let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        assert_ne!(after.paused, before.paused, "pause state should toggle");
        assert_eq!(
            after.total_minted, before.total_minted,
            "pause should not mint"
        );
        assert_eq!(
            after.total_burned, before.total_burned,
            "pause should not burn"
        );
    }

    fn lifecycle_freeze_toggle(&mut self) {
        let scenario = self.scenario();
        let slot = scenario.random_user_slot(&mut self.trident);
        let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        let ix = if before.frozen_for_slot(slot) {
            thaw_account_ix(
                scenario.authority,
                scenario.mint,
                scenario.tracked_account(slot),
            )
        } else {
            freeze_account_ix(
                scenario.authority,
                scenario.mint,
                scenario.tracked_account(slot),
            )
        };
        let result = process_ix!(&mut self.trident, ix, "FreezeToggle");
        assert!(
            result.is_success(),
            "freeze toggle should succeed: {}",
            result.logs()
        );

        let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        assert_ne!(
            after.frozen_for_slot(slot),
            before.frozen_for_slot(slot),
            "freeze state should toggle"
        );
        assert_eq!(
            after.total_minted, before.total_minted,
            "freeze should not mint"
        );
        assert_eq!(
            after.total_burned, before.total_burned,
            "freeze should not burn"
        );
    }

    fn lifecycle_burn(&mut self) {
        let scenario = self.scenario();
        if scenario.config(&mut self.trident).paused {
            let unpause = process_ix!(
                &mut self.trident,
                unpause_ix(scenario.authority, scenario.mint),
                "UnpauseBeforeBurn"
            );
            assert!(
                unpause.is_success(),
                "unpause before burn should succeed: {}",
                unpause.logs()
            );
        }

        let treasury_balance = scenario.token_amount(&mut self.trident, TokenSlot::Treasury);
        if treasury_balance == 0 {
            let refill = process_ix!(
                &mut self.trident,
                mint_ix(scenario.authority, scenario.mint, scenario.treasury, 100),
                "RefillTreasury"
            );
            assert!(
                refill.is_success(),
                "refill mint should succeed: {}",
                refill.logs()
            );
        }

        let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        let amount = self
            .trident
            .random_from_range(1_u64..=before.treasury_balance.max(1).min(500));
        let result = process_ix!(
            &mut self.trident,
            burn_ix(scenario.authority, scenario.mint, scenario.treasury, amount),
            "Burn"
        );
        assert!(
            result.is_success(),
            "burn should succeed: {}",
            result.logs()
        );

        let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        assert_eq!(
            after.total_burned,
            before.total_burned + amount,
            "burn counter drifted"
        );
        assert_eq!(
            after.total_minted, before.total_minted,
            "mint counter changed during burn"
        );
        assert_eq!(
            after.treasury_balance,
            before.treasury_balance - amount,
            "treasury balance should decrease after burn"
        );
        assert_eq!(
            after.supply,
            before.supply - amount,
            "supply should decrease after burn"
        );
    }

    fn lifecycle_blacklist_or_seize(&mut self) {
        Self::debug_log("flow=lifecycle_blacklist_or_seize");
        let scenario = self.scenario();
        if scenario.preset == state_model::PresetKind::Sss1 {
            self.lifecycle_freeze_toggle();
            return;
        }

        if self.trident.random_bool() {
            let slot = scenario.random_user_slot(&mut self.trident);
            let wallet = scenario.tracked_owner(slot);
            let exists = scenario
                .blacklist_entry(&mut self.trident, wallet)
                .is_some();
            let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);
            let ix = if exists {
                remove_from_blacklist_ix(scenario.authority, scenario.mint, wallet)
            } else {
                add_to_blacklist_ix(
                    scenario.authority,
                    scenario.mint,
                    wallet,
                    "fuzz".to_string(),
                )
            };
            let result = process_ix!(&mut self.trident, ix, "BlacklistToggle");
            assert!(
                result.is_success(),
                "blacklist toggle should succeed: {}",
                result.logs()
            );

            let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
            match slot {
                TokenSlot::UserA => assert_ne!(
                    after.user_a_blacklisted, before.user_a_blacklisted,
                    "user_a blacklist should toggle"
                ),
                TokenSlot::UserB => assert_ne!(
                    after.user_b_blacklisted, before.user_b_blacklisted,
                    "user_b blacklist should toggle"
                ),
                TokenSlot::Treasury => unreachable!("treasury is not blacklisted"),
            }
        } else {
            self.lifecycle_seize();
        }
    }

    fn lifecycle_seize(&mut self) {
        Self::debug_log("flow=lifecycle_seize");
        let scenario = self.scenario();
        if scenario.config(&mut self.trident).paused {
            let unpause = process_ix!(
                &mut self.trident,
                unpause_ix(scenario.authority, scenario.mint),
                "UnpauseBeforeSeize"
            );
            assert!(
                unpause.is_success(),
                "unpause before seize should succeed: {}",
                unpause.logs()
            );
        }

        if scenario
            .blacklist_entry(&mut self.trident, scenario.user_a)
            .is_none()
        {
            let add = process_ix!(
                &mut self.trident,
                add_to_blacklist_ix(
                    scenario.authority,
                    scenario.mint,
                    scenario.user_a,
                    "seize-target".to_string(),
                ),
                "BlacklistSeizeTarget"
            );
            assert!(
                add.is_success(),
                "blacklist before seize should succeed: {}",
                add.logs()
            );
        }

        if !scenario.token_is_frozen(&mut self.trident, TokenSlot::UserA) {
            let freeze = process_ix!(
                &mut self.trident,
                freeze_account_ix(scenario.authority, scenario.mint, scenario.user_a_account),
                "FreezeSeizeTarget"
            );
            assert!(
                freeze.is_success(),
                "freeze before seize should succeed: {}",
                freeze.logs()
            );
        }

        if scenario.token_amount(&mut self.trident, TokenSlot::UserA) == 0 {
            let refill = process_ix!(
                &mut self.trident,
                mint_ix(
                    scenario.authority,
                    scenario.mint,
                    scenario.user_a_account,
                    100
                ),
                "RefillSeizeTarget"
            );
            assert!(
                refill.is_success(),
                "refill before seize should succeed: {}",
                refill.logs()
            );
        }

        let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        let amount = self
            .trident
            .random_from_range(1_u64..=before.user_a_balance.max(1).min(250));
        let result = process_ix!(
            &mut self.trident,
            seize_ix_with_amount(
                scenario.authority,
                scenario.mint,
                scenario.user_a_account,
                scenario.treasury,
                scenario.user_a,
                scenario.authority,
                amount,
            ),
            "Seize"
        );
        assert!(
            result.is_success(),
            "seize should succeed: {}",
            result.logs()
        );

        let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        assert_eq!(
            after.user_a_balance,
            before.user_a_balance - amount,
            "seized account should lose balance"
        );
        assert_eq!(
            after.treasury_balance,
            before.treasury_balance + amount,
            "treasury should receive seized funds"
        );
        assert!(after.user_a_frozen, "seized account should end frozen");
        assert_eq!(after.supply, before.supply, "seize should preserve supply");
    }

    fn lifecycle_transfer_hook(&mut self) {
        Self::debug_log("flow=lifecycle_transfer_hook");
        let scenario = self.scenario();
        if scenario.preset == state_model::PresetKind::Sss1 {
            self.lifecycle_mint();
            return;
        }
        if scenario.config(&mut self.trident).paused {
            let unpause = process_ix!(
                &mut self.trident,
                unpause_ix(scenario.authority, scenario.mint),
                "UnpauseBeforeTransferHook"
            );
            assert!(
                unpause.is_success(),
                "unpause before transfer should succeed: {}",
                unpause.logs()
            );
        }

        for (slot, wallet, account, label) in [
            (TokenSlot::UserA, scenario.user_a, scenario.user_a_account, "PrepareTransferSource"),
            (
                TokenSlot::UserB,
                scenario.user_b,
                scenario.user_b_account,
                "PrepareTransferDestination",
            ),
        ] {
            if scenario.blacklist_entry(&mut self.trident, wallet).is_some() {
                let remove = process_ix!(
                    &mut self.trident,
                    remove_from_blacklist_ix(scenario.authority, scenario.mint, wallet),
                    label
                );
                assert!(
                    remove.is_success(),
                    "remove blacklist for {label} should succeed: {}",
                    remove.logs()
                );
            }
            if scenario.token_is_frozen(&mut self.trident, slot) {
                let thaw = process_ix!(
                    &mut self.trident,
                    thaw_account_ix(scenario.authority, scenario.mint, account),
                    label
                );
                assert!(
                    thaw.is_success(),
                    "thaw for {label} should succeed: {}",
                    thaw.logs()
                );
            }
        }

        if scenario.token_amount(&mut self.trident, TokenSlot::UserA) == 0 {
            let refill = process_ix!(
                &mut self.trident,
                mint_ix(scenario.authority, scenario.mint, scenario.user_a_account, 100),
                "RefillTransferSource"
            );
            assert!(
                refill.is_success(),
                "refill before transfer should succeed: {}",
                refill.logs()
            );
        }

        let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        let amount = self
            .trident
            .random_from_range(1_u64..=before.user_a_balance.max(1).min(250));
        let result = process_ix!(
            &mut self.trident,
            transfer_checked_with_hook_ix(
                scenario.user_a_account,
                scenario.mint,
                scenario.user_b_account,
                scenario.user_a,
                scenario.user_a,
                scenario.user_b,
                amount,
                6,
            ),
            "TransferHook"
        );
        assert!(
            result.is_success(),
            "transfer hook should succeed: {}",
            result.logs()
        );

        let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        assert_eq!(
            after.user_a_balance,
            before.user_a_balance - amount,
            "source balance should decrease"
        );
        assert_eq!(
            after.user_b_balance,
            before.user_b_balance + amount,
            "destination balance should increase"
        );
        assert_eq!(after.supply, before.supply, "transfer should preserve supply");
        assert_eq!(
            after.total_minted, before.total_minted,
            "transfer should not change minted counter"
        );
        assert_eq!(
            after.total_burned, before.total_burned,
            "transfer should not change burned counter"
        );
    }

    fn negative_unauthorized_pause(&mut self) {
        Self::debug_log("flow=negative_unauthorized_pause");
        let scenario = self.scenario();
        let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        let result = process_ix!(
            &mut self.trident,
            pause_ix(scenario.attacker, scenario.mint),
            "UnauthorizedPause"
        );
        assert_custom_error!(result, StablecoinError::NotPauser);
        let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        assert_snapshot_unchanged(&before, &after, "unauthorized pause");
    }

    fn negative_unauthorized_update_minter(&mut self) {
        Self::debug_log("flow=negative_unauthorized_update_minter");
        let scenario = self.scenario();
        let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        let result = process_ix!(
            &mut self.trident,
            update_minter_ix(
                scenario.attacker,
                scenario.mint,
                scenario.attacker,
                10_000,
                true,
            ),
            "UnauthorizedUpdateMinter"
        );
        assert_custom_error!(result, StablecoinError::NotMasterAuthority);
        let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        assert_snapshot_unchanged(&before, &after, "unauthorized update_minter");
    }

    fn negative_mint_while_paused(&mut self) {
        Self::debug_log("flow=negative_mint_while_paused");
        let scenario = self.scenario();
        if !scenario.config(&mut self.trident).paused {
            let pause = process_ix!(
                &mut self.trident,
                pause_ix(scenario.authority, scenario.mint),
                "PreparePausedMint"
            );
            assert!(
                pause.is_success(),
                "pause before negative mint should succeed: {}",
                pause.logs()
            );
        }
        let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        let result = process_ix!(
            &mut self.trident,
            mint_ix(scenario.authority, scenario.mint, scenario.treasury, 1),
            "MintWhilePaused"
        );
        assert_custom_error!(result, StablecoinError::StablecoinPaused);
        let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        assert_eq!(after.paused, true, "pause state should remain set");
        assert_snapshot_unchanged(&before, &after, "mint while paused");
    }

    fn negative_burn_while_paused(&mut self) {
        Self::debug_log("flow=negative_burn_while_paused");
        let scenario = self.scenario();
        if !scenario.config(&mut self.trident).paused {
            let pause = process_ix!(
                &mut self.trident,
                pause_ix(scenario.authority, scenario.mint),
                "PreparePausedBurn"
            );
            assert!(
                pause.is_success(),
                "pause before negative burn should succeed: {}",
                pause.logs()
            );
        }
        let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        let result = process_ix!(
            &mut self.trident,
            burn_ix(scenario.authority, scenario.mint, scenario.treasury, 1),
            "BurnWhilePaused"
        );
        assert_custom_error!(result, StablecoinError::StablecoinPaused);
        let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        assert_eq!(after.paused, true, "pause state should remain set");
        assert_snapshot_unchanged(&before, &after, "burn while paused");
    }

    fn negative_zero_amount_mint(&mut self) {
        Self::debug_log("flow=negative_zero_amount_mint");
        let scenario = self.scenario();
        if scenario.config(&mut self.trident).paused {
            let unpause = process_ix!(
                &mut self.trident,
                unpause_ix(scenario.authority, scenario.mint),
                "PrepareZeroMint"
            );
            assert!(
                unpause.is_success(),
                "unpause before zero mint should succeed: {}",
                unpause.logs()
            );
        }
        let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        let result = process_ix!(
            &mut self.trident,
            mint_ix(scenario.authority, scenario.mint, scenario.treasury, 0),
            "ZeroMint"
        );
        assert_custom_error!(result, StablecoinError::ZeroAmount);
        let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        assert_snapshot_unchanged(&before, &after, "zero amount mint");
    }

    fn negative_blacklist_or_seize(&mut self) {
        Self::debug_log("flow=negative_blacklist_or_seize");
        let scenario = self.scenario();
        let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);

        if scenario.preset == state_model::PresetKind::Sss1 {
            let result = process_ix!(
                &mut self.trident,
                add_to_blacklist_ix(
                    scenario.authority,
                    scenario.mint,
                    scenario.user_a,
                    "not-allowed".to_string(),
                ),
                "BlacklistOnSss1"
            );
            assert_custom_error!(result, StablecoinError::ComplianceNotEnabled);
        } else {
            if scenario.config(&mut self.trident).paused {
                let unpause = process_ix!(
                    &mut self.trident,
                    unpause_ix(scenario.authority, scenario.mint),
                    "PrepareNegativeSeizeUnpause"
                );
                assert!(
                    unpause.is_success(),
                    "unpause setup should succeed: {}",
                    unpause.logs()
                );
            }
            if !scenario
                .blacklist_entry(&mut self.trident, scenario.user_a)
                .is_some()
            {
                let add = process_ix!(
                    &mut self.trident,
                    add_to_blacklist_ix(
                        scenario.authority,
                        scenario.mint,
                        scenario.user_a,
                        "negative".to_string(),
                    ),
                    "PrepareNegativeSeizeBlacklist"
                );
                assert!(
                    add.is_success(),
                    "blacklist setup should succeed: {}",
                    add.logs()
                );
            }
            if scenario.token_is_frozen(&mut self.trident, TokenSlot::UserA) {
                let thaw = process_ix!(
                    &mut self.trident,
                    thaw_account_ix(scenario.authority, scenario.mint, scenario.user_a_account),
                    "PrepareNegativeSeizeThaw"
                );
                assert!(
                    thaw.is_success(),
                    "thaw setup should succeed: {}",
                    thaw.logs()
                );
            }

            let result = process_ix!(
                &mut self.trident,
                seize_ix_with_amount(
                    scenario.authority,
                    scenario.mint,
                    scenario.user_a_account,
                    scenario.treasury,
                    scenario.user_a,
                    scenario.authority,
                    1,
                ),
                "SeizeWithoutFrozenTarget"
            );
            assert_custom_error!(result, StablecoinError::TargetAccountNotFrozen);
        }

        let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        if scenario.preset == state_model::PresetKind::Sss1 {
            assert_snapshot_unchanged(&before, &after, "blacklist on sss1");
        } else {
            assert_eq!(
                after.total_minted, before.total_minted,
                "negative seize minted"
            );
            assert_eq!(
                after.total_burned, before.total_burned,
                "negative seize burned"
            );
            assert_eq!(after.supply, before.supply, "negative seize changed supply");
        }
    }

    fn negative_invalid_treasury_seize(&mut self) {
        Self::debug_log("flow=negative_invalid_treasury_seize");
        let scenario = self.scenario();
        if scenario.preset == state_model::PresetKind::Sss1 {
            self.negative_unauthorized_pause();
            return;
        }
        if scenario.config(&mut self.trident).paused {
            let unpause = process_ix!(
                &mut self.trident,
                unpause_ix(scenario.authority, scenario.mint),
                "PrepareInvalidTreasuryUnpause"
            );
            assert!(
                unpause.is_success(),
                "unpause before invalid treasury seize should succeed: {}",
                unpause.logs()
            );
        }
        if scenario
            .blacklist_entry(&mut self.trident, scenario.user_a)
            .is_none()
        {
            let add = process_ix!(
                &mut self.trident,
                add_to_blacklist_ix(
                    scenario.authority,
                    scenario.mint,
                    scenario.user_a,
                    "invalid-treasury".to_string(),
                ),
                "PrepareInvalidTreasuryBlacklist"
            );
            assert!(
                add.is_success(),
                "blacklist setup should succeed: {}",
                add.logs()
            );
        }
        if !scenario.token_is_frozen(&mut self.trident, TokenSlot::UserA) {
            let freeze = process_ix!(
                &mut self.trident,
                freeze_account_ix(scenario.authority, scenario.mint, scenario.user_a_account),
                "PrepareInvalidTreasuryFreeze"
            );
            assert!(
                freeze.is_success(),
                "freeze setup should succeed: {}",
                freeze.logs()
            );
        }
        if scenario.token_amount(&mut self.trident, TokenSlot::UserA) == 0 {
            let refill = process_ix!(
                &mut self.trident,
                mint_ix(
                    scenario.authority,
                    scenario.mint,
                    scenario.user_a_account,
                    25
                ),
                "PrepareInvalidTreasuryRefill"
            );
            assert!(
                refill.is_success(),
                "refill setup should succeed: {}",
                refill.logs()
            );
        }

        let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        let result = process_ix!(
            &mut self.trident,
            seize_ix_with_amount(
                scenario.authority,
                scenario.mint,
                scenario.user_a_account,
                scenario.user_b_account,
                scenario.user_a,
                scenario.user_b,
                1,
            ),
            "InvalidTreasurySeize"
        );
        assert_custom_error!(result, StablecoinError::InvalidTreasuryAccount);
        let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        assert_snapshot_unchanged(&before, &after, "invalid treasury seize");
    }

    fn negative_transfer_hook_blacklist(&mut self) {
        Self::debug_log("flow=negative_transfer_hook_blacklist");
        let scenario = self.scenario();
        if scenario.preset == state_model::PresetKind::Sss1 {
            self.negative_blacklist_or_seize();
            return;
        }
        if scenario.config(&mut self.trident).paused {
            let unpause = process_ix!(
                &mut self.trident,
                unpause_ix(scenario.authority, scenario.mint),
                "UnpauseBeforeNegativeTransferHook"
            );
            assert!(
                unpause.is_success(),
                "unpause before negative transfer should succeed: {}",
                unpause.logs()
            );
        }
        for (slot, account, label) in [
            (TokenSlot::UserA, scenario.user_a_account, "ThawNegativeTransferSource"),
            (
                TokenSlot::UserB,
                scenario.user_b_account,
                "ThawNegativeTransferDestination",
            ),
        ] {
            if scenario.token_is_frozen(&mut self.trident, slot) {
                let thaw = process_ix!(
                    &mut self.trident,
                    thaw_account_ix(scenario.authority, scenario.mint, account),
                    label
                );
                assert!(
                    thaw.is_success(),
                    "thaw setup for {label} should succeed: {}",
                    thaw.logs()
                );
            }
        }
        if scenario.token_amount(&mut self.trident, TokenSlot::UserA) == 0 {
            let refill = process_ix!(
                &mut self.trident,
                mint_ix(scenario.authority, scenario.mint, scenario.user_a_account, 10),
                "RefillNegativeTransferSource"
            );
            assert!(
                refill.is_success(),
                "refill before negative transfer should succeed: {}",
                refill.logs()
            );
        }

        let blacklist_sender = self.trident.random_bool();
        let (wallet, expected_error, label) = if blacklist_sender {
            (
                scenario.user_a,
                TransferHookError::SourceBlacklisted,
                "BlacklistNegativeTransferSource",
            )
        } else {
            (
                scenario.user_b,
                TransferHookError::DestinationBlacklisted,
                "BlacklistNegativeTransferDestination",
            )
        };
        for existing_wallet in [scenario.user_a, scenario.user_b] {
            if existing_wallet != wallet
                && scenario
                    .blacklist_entry(&mut self.trident, existing_wallet)
                    .is_some()
            {
                let remove = process_ix!(
                    &mut self.trident,
                    remove_from_blacklist_ix(scenario.authority, scenario.mint, existing_wallet),
                    "ClearOtherNegativeTransferBlacklist"
                );
                assert!(
                    remove.is_success(),
                    "clearing unrelated blacklist should succeed: {}",
                    remove.logs()
                );
            }
        }
        if scenario.blacklist_entry(&mut self.trident, wallet).is_none() {
            let add = process_ix!(
                &mut self.trident,
                add_to_blacklist_ix(scenario.authority, scenario.mint, wallet, "fuzz".to_string()),
                label
            );
            assert!(
                add.is_success(),
                "blacklist setup for negative transfer should succeed: {}",
                add.logs()
            );
        }

        let before = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        let result = process_ix!(
            &mut self.trident,
            transfer_checked_with_hook_ix(
                scenario.user_a_account,
                scenario.mint,
                scenario.user_b_account,
                scenario.user_a,
                scenario.user_a,
                scenario.user_b,
                1,
                6,
            ),
            "TransferHookBlacklisted"
        );
        assert_custom_error!(result, expected_error);
        let after = ScenarioSnapshot::capture(&mut self.trident, &scenario);
        assert_snapshot_unchanged(&before, &after, "transfer hook blacklist");
    }
}
