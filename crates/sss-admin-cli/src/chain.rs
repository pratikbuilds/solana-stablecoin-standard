use anyhow::{Context, Result};
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    system_program, sysvar,
    transaction::Transaction,
};
use spl_token_2022::{extension::StateWithExtensionsOwned, state::Account as TokenAccount};
use stablecoin::{
    instruction, instructions::initialize::InitializeParams, instructions::roles::UpdateMinterParams,
};
use std::path::Path;

use crate::{config::{InitConfigFile, Preset}, init::InitPlan};

const DEFAULT_MINTER_QUOTA: u64 = 1_000_000_000_000;

pub struct ChainClient {
    rpc: RpcClient,
    authority: Keypair,
}

pub struct InitExecution {
    pub mint: Pubkey,
    pub initialize_signature: Signature,
    pub minter_signature: Signature,
}

pub struct HolderRecord {
    pub owner: Pubkey,
    pub token_account: Pubkey,
    pub amount: u64,
}

pub struct MinterRecord {
    pub minter: Pubkey,
    pub quota: u64,
    pub minted: u64,
    pub active: bool,
}

impl ChainClient {
    pub fn from_runtime(config: Option<&InitConfigFile>) -> Result<Self> {
        let rpc_url = config
            .and_then(|cfg| cfg.rpc_url.clone())
            .or_else(|| std::env::var("SOLANA_RPC_URL").ok())
            .context("rpc_url must be set in config or SOLANA_RPC_URL for direct chain execution")?;
        Ok(Self {
            rpc: RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed()),
            authority: load_keypair(config.and_then(|cfg| cfg.authority_keypair.as_deref()))?,
        })
    }

    pub fn init(&self, plan: &InitPlan) -> Result<InitExecution> {
        let mint = Keypair::new();
        let initialize_signature = self.send_transaction(
            &[initialize_ix(self.authority.pubkey(), mint.pubkey(), &plan.config)],
            &[&mint],
        )?;
        let minter_signature = self.send_transaction(
            &[update_minter_ix(
                self.authority.pubkey(),
                mint.pubkey(),
                self.authority.pubkey(),
                DEFAULT_MINTER_QUOTA,
                true,
            )],
            &[],
        )?;
        Ok(InitExecution {
            mint: mint.pubkey(),
            initialize_signature,
            minter_signature,
        })
    }

    pub fn pause(&self, mint: Pubkey) -> Result<Signature> {
        self.send_transaction(&[pause_ix(self.authority.pubkey(), mint)], &[])
    }

    pub fn unpause(&self, mint: Pubkey) -> Result<Signature> {
        self.send_transaction(&[unpause_ix(self.authority.pubkey(), mint)], &[])
    }

    pub fn add_minter(&self, mint: Pubkey, minter: Pubkey, quota: u64) -> Result<Signature> {
        self.send_transaction(
            &[update_minter_ix(self.authority.pubkey(), mint, minter, quota, true)],
            &[],
        )
    }

    pub fn remove_minter(&self, mint: Pubkey, minter: Pubkey) -> Result<Signature> {
        self.send_transaction(
            &[update_minter_ix(self.authority.pubkey(), mint, minter, 0, false)],
            &[],
        )
    }

    pub fn list_minters(&self, mint: Pubkey) -> Result<Vec<MinterRecord>> {
        let accounts = self.rpc.get_program_accounts_with_config(
            &stablecoin::ID,
            RpcProgramAccountsConfig {
                filters: Some(vec![
                    RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                        0,
                        stablecoin_client::generated::accounts::MINTER_QUOTA_DISCRIMINATOR.to_vec(),
                    )),
                    RpcFilterType::Memcmp(Memcmp::new_raw_bytes(8, mint.to_bytes().to_vec())),
                ]),
                account_config: RpcAccountInfoConfig::default(),
                with_context: None,
                sort_results: None,
            },
        )?;

        let mut results = Vec::with_capacity(accounts.len());
        for (_, account) in accounts {
            let decoded = stablecoin_client::generated::accounts::MinterQuota::from_bytes(&account.data)
                .context("decode minter quota account")?;
            results.push(MinterRecord {
                minter: Pubkey::new_from_array(decoded.minter.to_bytes()),
                quota: decoded.quota,
                minted: decoded.minted,
                active: decoded.active,
            });
        }
        Ok(results)
    }

    pub fn list_holders(&self, mint: Pubkey, min_balance: Option<u64>) -> Result<Vec<HolderRecord>> {
        let accounts = self.rpc.get_program_accounts_with_config(
            &spl_token_2022::id(),
            RpcProgramAccountsConfig {
                filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                    0,
                    mint.to_bytes().to_vec(),
                ))]),
                account_config: RpcAccountInfoConfig::default(),
                with_context: None,
                sort_results: None,
            },
        )?;

        let mut holders = Vec::new();
        for (address, account) in accounts {
            let unpacked =
                StateWithExtensionsOwned::<TokenAccount>::unpack(account.data).context("decode token account")?;
            if unpacked.base.amount == 0 {
                continue;
            }
            if let Some(min_balance) = min_balance {
                if unpacked.base.amount < min_balance {
                    continue;
                }
            }
            holders.push(HolderRecord {
                owner: unpacked.base.owner,
                token_account: address,
                amount: unpacked.base.amount,
            });
        }
        holders.sort_by(|left, right| right.amount.cmp(&left.amount));
        Ok(holders)
    }

    fn send_transaction(&self, instructions: &[Instruction], extra_signers: &[&Keypair]) -> Result<Signature> {
        let recent = self.rpc.get_latest_blockhash().context("get latest blockhash")?;
        let mut signers: Vec<&Keypair> = vec![&self.authority];
        signers.extend_from_slice(extra_signers);
        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(&self.authority.pubkey()),
            &signers,
            recent,
        );
        self.rpc
            .send_and_confirm_transaction(&tx)
            .context("send and confirm transaction")
    }
}

fn initialize_ix(authority: Pubkey, mint: Pubkey, config: &InitConfigFile) -> Instruction {
    let params = InitializeParams {
        name: config.name.clone(),
        symbol: config.symbol.clone(),
        uri: config.uri.clone(),
        decimals: config.decimals,
        enable_permanent_delegate: config.features.enable_permanent_delegate,
        enable_transfer_hook: config.features.enable_transfer_hook,
        default_account_frozen: config.features.default_account_frozen,
    };
    let accounts = stablecoin::accounts::Initialize {
        authority,
        mint,
        config: config_pda(&mint),
        role_config: roles_pda(&mint),
        extra_account_meta_list: (config.preset == Preset::Sss2).then_some(extra_account_meta_list_pda(&mint)),
        transfer_hook_program: (config.preset == Preset::Sss2).then_some(transfer_hook::ID),
        token_program: spl_token_2022::id(),
        system_program: system_program::ID,
        rent: sysvar::rent::ID,
        event_authority: event_authority_pda(),
        program: stablecoin::ID,
    };

    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: instruction::Initialize { params }.data(),
    }
}

fn update_minter_ix(authority: Pubkey, mint: Pubkey, minter: Pubkey, quota: u64, active: bool) -> Instruction {
    let accounts = stablecoin::accounts::UpdateMinter {
        authority,
        config: config_pda(&mint),
        role_config: roles_pda(&mint),
        mint,
        minter,
        minter_quota: minter_quota_pda(&mint, &minter),
        system_program: system_program::ID,
        event_authority: event_authority_pda(),
        program: stablecoin::ID,
    };

    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: instruction::UpdateMinter {
            params: UpdateMinterParams {
                minter,
                quota,
                active,
            },
        }
        .data(),
    }
}

fn pause_ix(authority: Pubkey, mint: Pubkey) -> Instruction {
    let accounts = stablecoin::accounts::PauseOps {
        authority,
        config: config_pda(&mint),
        role_config: roles_pda(&mint),
        event_authority: event_authority_pda(),
        program: stablecoin::ID,
    };
    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: instruction::Pause {}.data(),
    }
}

fn unpause_ix(authority: Pubkey, mint: Pubkey) -> Instruction {
    let accounts = stablecoin::accounts::UnpauseOps {
        authority,
        config: config_pda(&mint),
        role_config: roles_pda(&mint),
        event_authority: event_authority_pda(),
        program: stablecoin::ID,
    };
    Instruction {
        program_id: stablecoin::ID,
        accounts: accounts.to_account_metas(None),
        data: instruction::Unpause {}.data(),
    }
}

fn config_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[sss_common::SEED_CONFIG, mint.as_ref()], &stablecoin::ID).0
}

fn roles_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[sss_common::SEED_ROLES, mint.as_ref()], &stablecoin::ID).0
}

fn minter_quota_pda(mint: &Pubkey, minter: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[sss_common::SEED_MINTER, mint.as_ref(), minter.as_ref()],
        &stablecoin::ID,
    )
    .0
}

fn extra_account_meta_list_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[sss_common::SEED_EXTRA_ACCOUNT_METAS, mint.as_ref()],
        &transfer_hook::ID,
    )
    .0
}

fn event_authority_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], &stablecoin::ID).0
}

fn load_keypair(config_path: Option<&str>) -> Result<Keypair> {
    if let Some(path) = config_path {
        return load_keypair_file(Path::new(path));
    }
    if let Ok(path) = std::env::var("SSS_AUTHORITY_KEYPAIR") {
        return load_keypair_file(Path::new(&path));
    }
    let default_path = dirs::home_dir()
        .map(|home| home.join(".config/solana/id.json"))
        .context("home directory not found for default Solana keypair")?;
    load_keypair_file(&default_path)
}

fn load_keypair_file(path: &Path) -> Result<Keypair> {
    let contents =
        std::fs::read_to_string(path).with_context(|| format!("read keypair file {}", path.display()))?;
    let bytes: Vec<u8> =
        serde_json::from_str(&contents).with_context(|| format!("parse keypair file {}", path.display()))?;
    Keypair::from_bytes(&bytes).context("invalid authority keypair")
}
