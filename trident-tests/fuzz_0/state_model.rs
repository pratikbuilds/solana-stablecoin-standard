use anchor_lang::{solana_program::instruction::AccountMeta, AccountDeserialize, InstructionData, ToAccountMetas};
use stablecoin::instructions::initialize::InitializeParams;
use stablecoin::instructions::roles::UpdateMinterParams;
use stablecoin::state::{BlacklistEntry, MinterQuota, RoleConfig, StablecoinConfig};
use trident_fuzz::fuzzing::*;

use sss_common::{SEED_BLACKLIST, SEED_CONFIG, SEED_EXTRA_ACCOUNT_METAS, SEED_MINTER, SEED_ROLES};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresetKind {
    Sss1,
    Sss2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenSlot {
    Treasury,
    UserA,
    UserB,
}

#[derive(Clone, Copy, Debug)]
pub struct ScenarioState {
    pub preset: PresetKind,
    pub authority: Pubkey,
    pub minter: Pubkey,
    pub user_a: Pubkey,
    pub user_b: Pubkey,
    pub attacker: Pubkey,
    pub mint: Pubkey,
    pub config: Pubkey,
    pub role_config: Pubkey,
    pub treasury: Pubkey,
    pub user_a_account: Pubkey,
    pub user_b_account: Pubkey,
    pub last_total_minted: u64,
    pub last_total_burned: u64,
}

impl ScenarioState {
    pub fn initialize(trident: &mut Trident) -> Self {
        let preset = if trident.random_bool() {
            PresetKind::Sss1
        } else {
            PresetKind::Sss2
        };

        let authority = trident.random_pubkey();
        let minter = trident.random_pubkey();
        let user_a = trident.random_pubkey();
        let user_b = trident.random_pubkey();
        let attacker = trident.random_pubkey();
        let mint = trident.random_pubkey();

        for actor in [authority, minter, user_a, user_b, attacker] {
            trident.airdrop(&actor, 10 * LAMPORTS_PER_SOL);
        }

        let params = match preset {
            PresetKind::Sss1 => InitializeParams {
                name: trident.random_string(10),
                symbol: "SSS1".to_string(),
                uri: "https://example.com/sss1.json".to_string(),
                decimals: 6,
                enable_permanent_delegate: false,
                enable_transfer_hook: false,
                default_account_frozen: false,
            },
            PresetKind::Sss2 => InitializeParams {
                name: trident.random_string(10),
                symbol: "SSS2".to_string(),
                uri: "https://example.com/sss2.json".to_string(),
                decimals: 6,
                enable_permanent_delegate: true,
                enable_transfer_hook: true,
                default_account_frozen: false,
            },
        };

        let init_result = trident.process_transaction(
            &[initialize_ix(authority, mint, params)],
            Some("Initialize"),
        );
        assert!(
            init_result.is_success(),
            "initialize should succeed: {}",
            init_result.logs()
        );

        let treasury = trident.random_pubkey();
        let user_a_account = trident.random_pubkey();
        let user_b_account = trident.random_pubkey();

        for (label, token_account, owner) in [
            ("CreateTreasury", treasury, authority),
            ("CreateUserA", user_a_account, user_a),
            ("CreateUserB", user_b_account, user_b),
        ] {
            let ixs = trident.initialize_token_account_2022(
                &authority,
                &token_account,
                &mint,
                &owner,
                &[],
            );
            let result = trident.process_transaction(&ixs, Some(label));
            assert!(
                result.is_success(),
                "{label} should succeed: {}",
                result.logs()
            );
        }

        for (target, quota) in [(authority, 500_000_u64), (minter, 250_000_u64)] {
            let result = trident.process_transaction(
                &[update_minter_ix(authority, mint, target, quota, true)],
                Some("SeedMinterQuota"),
            );
            assert!(
                result.is_success(),
                "update_minter should succeed for {target}: {}",
                result.logs()
            );
        }

        for (destination, amount, label) in [
            (treasury, 10_000_u64, "SeedTreasuryBalance"),
            (user_a_account, 6_000_u64, "SeedUserABalance"),
        ] {
            let result = trident.process_transaction(
                &[mint_ix(authority, mint, destination, amount)],
                Some(label),
            );
            assert!(
                result.is_success(),
                "{label} should succeed: {}",
                result.logs()
            );
        }

        let config = config(trident, &config_pda(&mint)).expect("config should exist");

        Self {
            preset,
            authority,
            minter,
            user_a,
            user_b,
            attacker,
            mint,
            config: config_pda(&mint),
            role_config: roles_pda(&mint),
            treasury,
            user_a_account,
            user_b_account,
            last_total_minted: config.total_minted,
            last_total_burned: config.total_burned,
        }
    }

    pub fn tracked_account(&self, slot: TokenSlot) -> Pubkey {
        match slot {
            TokenSlot::Treasury => self.treasury,
            TokenSlot::UserA => self.user_a_account,
            TokenSlot::UserB => self.user_b_account,
        }
    }

    pub fn tracked_owner(&self, slot: TokenSlot) -> Pubkey {
        match slot {
            TokenSlot::Treasury => self.authority,
            TokenSlot::UserA => self.user_a,
            TokenSlot::UserB => self.user_b,
        }
    }

    pub fn random_slot(&self, trident: &mut Trident) -> TokenSlot {
        match trident.random_from_range(0..3) {
            0 => TokenSlot::Treasury,
            1 => TokenSlot::UserA,
            _ => TokenSlot::UserB,
        }
    }

    pub fn random_user_slot(&self, trident: &mut Trident) -> TokenSlot {
        match trident.random_from_range(0..2) {
            0 => TokenSlot::UserA,
            _ => TokenSlot::UserB,
        }
    }

    pub fn config(&self, trident: &mut Trident) -> StablecoinConfig {
        config(trident, &self.config).expect("config should deserialize")
    }

    pub fn roles(&self, trident: &mut Trident) -> RoleConfig {
        roles(trident, &self.role_config).expect("role config should deserialize")
    }

    pub fn quota(&self, trident: &mut Trident, owner: Pubkey) -> Option<MinterQuota> {
        minter_quota(trident, &minter_quota_pda(&self.mint, &owner))
    }

    pub fn blacklist_entry(&self, trident: &mut Trident, wallet: Pubkey) -> Option<BlacklistEntry> {
        blacklist_entry(trident, &blacklist_pda(&self.mint, &wallet))
    }

    pub fn token_amount(&self, trident: &mut Trident, slot: TokenSlot) -> u64 {
        trident
            .get_token_account(self.tracked_account(slot))
            .expect("token account should exist")
            .account
            .amount
    }

    pub fn token_is_frozen(&self, trident: &mut Trident, slot: TokenSlot) -> bool {
        trident
            .get_token_account(self.tracked_account(slot))
            .expect("token account should exist")
            .account
            .state as u8
            == 2
    }

    pub fn mint_supply(&self, trident: &mut Trident) -> u64 {
        trident
            .get_mint(self.mint)
            .expect("mint should exist")
            .mint
            .supply
    }
}

pub fn config_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[SEED_CONFIG, mint.as_ref()], &stablecoin::ID).0
}

pub fn roles_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[SEED_ROLES, mint.as_ref()], &stablecoin::ID).0
}

pub fn minter_quota_pda(mint: &Pubkey, minter: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[SEED_MINTER, mint.as_ref(), minter.as_ref()],
        &stablecoin::ID,
    )
    .0
}

pub fn blacklist_pda(mint: &Pubkey, wallet: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[SEED_BLACKLIST, mint.as_ref(), wallet.as_ref()],
        &stablecoin::ID,
    )
    .0
}

pub fn extra_account_meta_list_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[SEED_EXTRA_ACCOUNT_METAS, mint.as_ref()],
        &transfer_hook::ID,
    )
    .0
}

pub fn initialize_ix(authority: Pubkey, mint: Pubkey, params: InitializeParams) -> Instruction {
    let accounts = stablecoin::accounts::Initialize {
        authority,
        mint,
        config: config_pda(&mint),
        role_config: roles_pda(&mint),
        extra_account_meta_list: params
            .enable_transfer_hook
            .then_some(extra_account_meta_list_pda(&mint)),
        transfer_hook_program: params.enable_transfer_hook.then_some(transfer_hook::ID),
        token_program: spl_token_2022::id(),
        system_program: solana_sdk::system_program::ID,
        rent: solana_sdk::sysvar::rent::ID,
    };

    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: stablecoin::instruction::Initialize { params }.data(),
    }
}

pub fn update_minter_ix(
    authority: Pubkey,
    mint: Pubkey,
    minter: Pubkey,
    quota: u64,
    active: bool,
) -> Instruction {
    let accounts = stablecoin::accounts::UpdateMinter {
        authority,
        config: config_pda(&mint),
        role_config: roles_pda(&mint),
        mint,
        minter,
        minter_quota: minter_quota_pda(&mint, &minter),
        system_program: solana_sdk::system_program::ID,
    };

    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: stablecoin::instruction::UpdateMinter {
            params: UpdateMinterParams {
                minter,
                quota,
                active,
            },
        }
        .data(),
    }
}

pub fn mint_ix(authority: Pubkey, mint: Pubkey, to: Pubkey, amount: u64) -> Instruction {
    let accounts = stablecoin::accounts::MintTokens {
        authority,
        config: config_pda(&mint),
        minter_quota: minter_quota_pda(&mint, &authority),
        mint,
        to,
        token_program: spl_token_2022::id(),
    };

    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: stablecoin::instruction::Mint { amount }.data(),
    }
}

pub fn pause_ix(authority: Pubkey, mint: Pubkey) -> Instruction {
    let accounts = stablecoin::accounts::PauseOps {
        authority,
        config: config_pda(&mint),
        role_config: roles_pda(&mint),
    };

    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: stablecoin::instruction::Pause {}.data(),
    }
}

pub fn unpause_ix(authority: Pubkey, mint: Pubkey) -> Instruction {
    let accounts = stablecoin::accounts::UnpauseOps {
        authority,
        config: config_pda(&mint),
        role_config: roles_pda(&mint),
    };

    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: stablecoin::instruction::Unpause {}.data(),
    }
}

pub fn burn_ix(authority: Pubkey, mint: Pubkey, from: Pubkey, amount: u64) -> Instruction {
    let accounts = stablecoin::accounts::BurnTokens {
        authority,
        config: config_pda(&mint),
        role_config: roles_pda(&mint),
        mint,
        from,
        token_program: spl_token_2022::id(),
    };

    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: stablecoin::instruction::Burn { amount }.data(),
    }
}

pub fn add_to_blacklist_ix(
    authority: Pubkey,
    mint: Pubkey,
    wallet: Pubkey,
    reason: String,
) -> Instruction {
    let accounts = stablecoin::accounts::AddToBlacklist {
        authority,
        config: config_pda(&mint),
        role_config: roles_pda(&mint),
        wallet,
        blacklist_entry: blacklist_pda(&mint, &wallet),
        system_program: solana_sdk::system_program::ID,
    };

    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: stablecoin::instruction::AddToBlacklist { reason }.data(),
    }
}

pub fn remove_from_blacklist_ix(authority: Pubkey, mint: Pubkey, wallet: Pubkey) -> Instruction {
    let accounts = stablecoin::accounts::RemoveFromBlacklist {
        authority,
        config: config_pda(&mint),
        role_config: roles_pda(&mint),
        blacklist_entry: blacklist_pda(&mint, &wallet),
    };

    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: stablecoin::instruction::RemoveFromBlacklist {}.data(),
    }
}

pub fn freeze_account_ix(authority: Pubkey, mint: Pubkey, account: Pubkey) -> Instruction {
    let accounts = stablecoin::accounts::FreezeTokenAccount {
        authority,
        config: config_pda(&mint),
        role_config: roles_pda(&mint),
        mint,
        account,
        token_program: spl_token_2022::id(),
    };

    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: stablecoin::instruction::FreezeAccount {}.data(),
    }
}

pub fn thaw_account_ix(authority: Pubkey, mint: Pubkey, account: Pubkey) -> Instruction {
    let accounts = stablecoin::accounts::ThawTokenAccount {
        authority,
        config: config_pda(&mint),
        role_config: roles_pda(&mint),
        mint,
        account,
        token_program: spl_token_2022::id(),
    };

    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: stablecoin::instruction::ThawAccount {}.data(),
    }
}

pub fn seize_ix_with_amount(
    authority: Pubkey,
    mint: Pubkey,
    from: Pubkey,
    to: Pubkey,
    victim_wallet: Pubkey,
    treasury_owner: Pubkey,
    amount: u64,
) -> Instruction {
    let accounts = stablecoin::accounts::SeizeTokens {
        authority,
        config: config_pda(&mint),
        role_config: roles_pda(&mint),
        mint,
        from,
        to,
        blacklist_entry: blacklist_pda(&mint, &victim_wallet),
        stablecoin_program: stablecoin::ID,
        transfer_hook_program: transfer_hook::ID,
        extra_account_meta_list: extra_account_meta_list_pda(&mint),
        destination_blacklist: blacklist_pda(&mint, &treasury_owner),
        token_program: spl_token_2022::id(),
    };

    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: stablecoin::instruction::Seize { amount }.data(),
    }
}

pub fn transfer_checked_with_hook_ix(
    source: Pubkey,
    mint: Pubkey,
    destination: Pubkey,
    authority: Pubkey,
    source_owner: Pubkey,
    destination_owner: Pubkey,
    amount: u64,
    decimals: u8,
) -> Instruction {
    let mut ix = spl_token_2022::instruction::transfer_checked(
        &spl_token_2022::id(),
        &source,
        &mint,
        &destination,
        &authority,
        &[],
        amount,
        decimals,
    )
    .expect("transfer_checked instruction");

    ix.accounts.extend([
        AccountMeta::new_readonly(stablecoin::ID, false),
        AccountMeta::new_readonly(config_pda(&mint), false),
        AccountMeta::new_readonly(blacklist_pda(&mint, &source_owner), false),
        AccountMeta::new_readonly(blacklist_pda(&mint, &destination_owner), false),
        AccountMeta::new_readonly(extra_account_meta_list_pda(&mint), false),
        AccountMeta::new_readonly(transfer_hook::ID, false),
    ]);

    ix
}

fn deserialize_anchor_account<T: AccountDeserialize>(
    trident: &mut Trident,
    pubkey: &Pubkey,
) -> Option<T> {
    let account = trident.get_account(pubkey);
    let mut data = account.data();
    if data.is_empty() {
        return None;
    }
    T::try_deserialize(&mut data).ok()
}

pub fn config(trident: &mut Trident, pubkey: &Pubkey) -> Option<StablecoinConfig> {
    deserialize_anchor_account(trident, pubkey)
}

pub fn roles(trident: &mut Trident, pubkey: &Pubkey) -> Option<RoleConfig> {
    deserialize_anchor_account(trident, pubkey)
}

pub fn minter_quota(trident: &mut Trident, pubkey: &Pubkey) -> Option<MinterQuota> {
    deserialize_anchor_account(trident, pubkey)
}

pub fn blacklist_entry(trident: &mut Trident, pubkey: &Pubkey) -> Option<BlacklistEntry> {
    deserialize_anchor_account(trident, pubkey)
}
