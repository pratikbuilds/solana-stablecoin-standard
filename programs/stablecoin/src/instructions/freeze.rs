use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    freeze_account, thaw_account, FreezeAccount, Mint, ThawAccount, TokenAccount, TokenInterface,
};

use sss_common::{SEED_CONFIG, SEED_ROLES};

use crate::errors::StablecoinError;
use crate::events::{AccountFrozen, AccountThawed};
use crate::state::{RoleConfig, StablecoinConfig};

#[derive(Accounts)]
pub struct FreezeTokenAccount<'info> {
    pub authority: Signer<'info>,
    #[account(
        seeds = [SEED_CONFIG, mint.key().as_ref()],
        bump = config.bump
    )]
    pub config: Account<'info, StablecoinConfig>,
    #[account(
        seeds = [SEED_ROLES, mint.key().as_ref()],
        bump = role_config.bump
    )]
    pub role_config: Account<'info, RoleConfig>,
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        token::mint = mint,
        token::token_program = token_program
    )]
    pub account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct ThawTokenAccount<'info> {
    pub authority: Signer<'info>,
    #[account(
        seeds = [SEED_CONFIG, mint.key().as_ref()],
        bump = config.bump
    )]
    pub config: Account<'info, StablecoinConfig>,
    #[account(
        seeds = [SEED_ROLES, mint.key().as_ref()],
        bump = role_config.bump
    )]
    pub role_config: Account<'info, RoleConfig>,
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        token::mint = mint,
        token::token_program = token_program
    )]
    pub account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn freeze_handler(ctx: Context<FreezeTokenAccount>) -> Result<()> {
    require!(
        ctx.accounts
            .role_config
            .is_pauser(&ctx.accounts.authority.key()),
        StablecoinError::NotPauser
    );

    let mint_key = ctx.accounts.mint.key();
    let bump = ctx.accounts.config.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[SEED_CONFIG, mint_key.as_ref(), &[bump]]];

    freeze_account(CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        FreezeAccount {
            mint: ctx.accounts.mint.to_account_info(),
            account: ctx.accounts.account.to_account_info(),
            authority: ctx.accounts.config.to_account_info(),
        },
        signer_seeds,
    ))?;

    emit!(AccountFrozen {
        mint: ctx.accounts.mint.key(),
        account: ctx.accounts.account.key(),
        authority: ctx.accounts.authority.key(),
    });

    Ok(())
}

pub fn thaw_handler(ctx: Context<ThawTokenAccount>) -> Result<()> {
    require!(
        ctx.accounts
            .role_config
            .is_pauser(&ctx.accounts.authority.key()),
        StablecoinError::NotPauser
    );

    let mint_key = ctx.accounts.mint.key();
    let bump = ctx.accounts.config.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[SEED_CONFIG, mint_key.as_ref(), &[bump]]];

    thaw_account(CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        ThawAccount {
            mint: ctx.accounts.mint.to_account_info(),
            account: ctx.accounts.account.to_account_info(),
            authority: ctx.accounts.config.to_account_info(),
        },
        signer_seeds,
    ))?;

    emit!(AccountThawed {
        mint: ctx.accounts.mint.key(),
        account: ctx.accounts.account.key(),
        authority: ctx.accounts.authority.key(),
    });

    Ok(())
}
