#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount};
use spl_tlv_account_resolution::account::ExtraAccountMeta;
use spl_tlv_account_resolution::seeds::Seed;
use spl_tlv_account_resolution::state::ExtraAccountMetaList;
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

use sss_common::{SEED_BLACKLIST, SEED_CONFIG, SEED_EXTRA_ACCOUNT_METAS};

declare_id!("6QNzPyTwg2MH778GL8idYiU3teFJiuQx6R5L7xdU17KC");

const STABLECOIN_PROGRAM_ID: Pubkey = pubkey!("Gbq8ZoZ4fE2J8wywFDYgSREPWL5qhtaneAX9PwQuQyCC");
const INITIALIZE_EXTRA_ACCOUNT_META_LIST_DISCRIMINATOR: [u8; 8] =
    [43, 34, 13, 49, 167, 88, 235, 235];
const EXECUTE_DISCRIMINATOR: [u8; 8] = [105, 37, 101, 197, 75, 251, 102, 26];
const CORE_PROGRAM_INDEX: u8 = 5;
const MINT_ACCOUNT_INDEX: u8 = 1;
const SOURCE_TOKEN_ACCOUNT_INDEX: u8 = 0;
const DESTINATION_TOKEN_ACCOUNT_INDEX: u8 = 2;
const TOKEN_ACCOUNT_OWNER_OFFSET: u8 = 32;
const TOKEN_ACCOUNT_OWNER_LENGTH: u8 = 32;

#[program]
pub mod transfer_hook {
    use super::*;

    #[instruction(discriminator = &INITIALIZE_EXTRA_ACCOUNT_META_LIST_DISCRIMINATOR)]
    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
    ) -> Result<()> {
        let extra_metas = build_extra_account_metas()?;
        let mut data = ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?;
        ExtraAccountMetaList::init::<ExecuteInstruction>(&mut data, &extra_metas)
            .map_err(|_| error!(TransferHookError::InvalidExtraAccountMetaList))?;

        Ok(())
    }

    #[instruction(discriminator = &EXECUTE_DISCRIMINATOR)]
    pub fn transfer_hook(ctx: Context<TransferHook>, _amount: u64) -> Result<()> {
        let expected_config = Pubkey::find_program_address(
            &[SEED_CONFIG, ctx.accounts.mint.key().as_ref()],
            &STABLECOIN_PROGRAM_ID,
        )
        .0;
        require!(
            ctx.accounts.stablecoin_program.key() == STABLECOIN_PROGRAM_ID,
            TransferHookError::InvalidStablecoinProgram
        );
        require!(
            ctx.accounts.config.key() == expected_config,
            TransferHookError::InvalidConfigAccount
        );

        let expected_source_blacklist = Pubkey::find_program_address(
            &[
                SEED_BLACKLIST,
                ctx.accounts.mint.key().as_ref(),
                ctx.accounts.source.owner.as_ref(),
            ],
            &STABLECOIN_PROGRAM_ID,
        )
        .0;
        let expected_destination_blacklist = Pubkey::find_program_address(
            &[
                SEED_BLACKLIST,
                ctx.accounts.mint.key().as_ref(),
                ctx.accounts.destination.owner.as_ref(),
            ],
            &STABLECOIN_PROGRAM_ID,
        )
        .0;

        require!(
            ctx.accounts.source_blacklist.key() == expected_source_blacklist,
            TransferHookError::InvalidBlacklistAccount
        );
        require!(
            ctx.accounts.destination_blacklist.key() == expected_destination_blacklist,
            TransferHookError::InvalidBlacklistAccount
        );

        if ctx.accounts.authority.key() == ctx.accounts.config.key() {
            return Ok(());
        }

        if ctx.accounts.source_blacklist.data_len() > 0 {
            return err!(TransferHookError::SourceBlacklisted);
        }

        if ctx.accounts.destination_blacklist.data_len() > 0 {
            return err!(TransferHookError::DestinationBlacklisted);
        }

        Ok(())
    }

    pub fn fallback<'info>(
        program_id: &Pubkey,
        accounts: &'info [AccountInfo<'info>],
        data: &[u8],
    ) -> Result<()> {
        let instruction =
            spl_transfer_hook_interface::instruction::TransferHookInstruction::unpack(data)?;

        match instruction {
            spl_transfer_hook_interface::instruction::TransferHookInstruction::Execute {
                amount,
            } => __private::__global::transfer_hook(program_id, accounts, &amount.to_le_bytes()),
            _ => err!(TransferHookError::InvalidInstruction),
        }
    }
}

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: PDA created and owned by this program.
    #[account(
        init,
        seeds = [SEED_EXTRA_ACCOUNT_METAS, mint.key().as_ref()],
        bump,
        payer = payer,
        space = ExtraAccountMetaList::size_of(build_extra_account_metas()?.len())?
    )]
    pub extra_account_meta_list: AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(token::mint = mint)]
    pub source: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(token::mint = mint)]
    pub destination: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: Source owner or delegate authority provided by Token-2022.
    pub authority: UncheckedAccount<'info>,
    /// CHECK: Extra account meta list PDA resolved by Token-2022.
    #[account(
        seeds = [SEED_EXTRA_ACCOUNT_METAS, mint.key().as_ref()],
        bump
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    /// CHECK: Stablecoin program id extra meta.
    pub stablecoin_program: UncheckedAccount<'info>,
    /// CHECK: Stablecoin config PDA extra meta.
    pub config: UncheckedAccount<'info>,
    /// CHECK: Source blacklist PDA extra meta.
    pub source_blacklist: UncheckedAccount<'info>,
    /// CHECK: Destination blacklist PDA extra meta.
    pub destination_blacklist: UncheckedAccount<'info>,
}

#[error_code]
pub enum TransferHookError {
    #[msg("Stablecoin program account is invalid")]
    InvalidStablecoinProgram,
    #[msg("Stablecoin config account is invalid")]
    InvalidConfigAccount,
    #[msg("Blacklist PDA account is invalid")]
    InvalidBlacklistAccount,
    #[msg("Source address is blacklisted")]
    SourceBlacklisted,
    #[msg("Destination address is blacklisted")]
    DestinationBlacklisted,
    #[msg("Extra account meta list is invalid")]
    InvalidExtraAccountMetaList,
    #[msg("Unsupported transfer hook instruction")]
    InvalidInstruction,
}

fn build_extra_account_metas() -> Result<Vec<ExtraAccountMeta>> {
    Ok(vec![
        ExtraAccountMeta::new_with_pubkey(&STABLECOIN_PROGRAM_ID, false, false)?,
        ExtraAccountMeta::new_external_pda_with_seeds(
            CORE_PROGRAM_INDEX,
            &[
                Seed::Literal {
                    bytes: SEED_CONFIG.to_vec(),
                },
                Seed::AccountKey {
                    index: MINT_ACCOUNT_INDEX,
                },
            ],
            false,
            false,
        )?,
        ExtraAccountMeta::new_external_pda_with_seeds(
            CORE_PROGRAM_INDEX,
            &[
                Seed::Literal {
                    bytes: SEED_BLACKLIST.to_vec(),
                },
                Seed::AccountKey {
                    index: MINT_ACCOUNT_INDEX,
                },
                Seed::AccountData {
                    account_index: SOURCE_TOKEN_ACCOUNT_INDEX,
                    data_index: TOKEN_ACCOUNT_OWNER_OFFSET,
                    length: TOKEN_ACCOUNT_OWNER_LENGTH,
                },
            ],
            false,
            false,
        )?,
        ExtraAccountMeta::new_external_pda_with_seeds(
            CORE_PROGRAM_INDEX,
            &[
                Seed::Literal {
                    bytes: SEED_BLACKLIST.to_vec(),
                },
                Seed::AccountKey {
                    index: MINT_ACCOUNT_INDEX,
                },
                Seed::AccountData {
                    account_index: DESTINATION_TOKEN_ACCOUNT_INDEX,
                    data_index: TOKEN_ACCOUNT_OWNER_OFFSET,
                    length: TOKEN_ACCOUNT_OWNER_LENGTH,
                },
            ],
            false,
            false,
        )?,
    ])
}
