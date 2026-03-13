use anchor_lang::prelude::*;
use anchor_spl::token_interface::{mint_to, Mint, MintTo, TokenAccount, TokenInterface};

use sss_common::{SEED_CONFIG, SEED_MINTER};

use crate::errors::StablecoinError;
use crate::events::TokensMinted;
use crate::state::{MinterQuota, StablecoinConfig};

#[event_cpi]
#[derive(Accounts)]
pub struct MintTokens<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_CONFIG, mint.key().as_ref()],
        bump = config.bump
    )]
    pub config: Account<'info, StablecoinConfig>,
    #[account(
        mut,
        seeds = [SEED_MINTER, mint.key().as_ref(), authority.key().as_ref()],
        bump = minter_quota.bump
    )]
    pub minter_quota: Account<'info, MinterQuota>,
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        token::mint = mint,
        token::token_program = token_program
    )]
    pub to: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn mint_handler(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
    require!(amount > 0, StablecoinError::ZeroAmount);
    require!(
        !ctx.accounts.config.paused,
        StablecoinError::StablecoinPaused
    );

    let minter_quota = &mut ctx.accounts.minter_quota;
    require!(
        minter_quota.mint == ctx.accounts.mint.key()
            && minter_quota.minter == ctx.accounts.authority.key()
            && minter_quota.can_mint(amount),
        StablecoinError::NotActiveMinter
    );

    let mint_key = ctx.accounts.mint.key();
    let bump = ctx.accounts.config.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[SEED_CONFIG, mint_key.as_ref(), &[bump]]];

    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.to.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
    )?;

    minter_quota.record_mint(amount)?;
    ctx.accounts.config.total_minted = ctx
        .accounts
        .config
        .total_minted
        .checked_add(amount)
        .ok_or(error!(StablecoinError::Overflow))?;

    emit_cpi!(TokensMinted {
        mint: ctx.accounts.mint.key(),
        to: ctx.accounts.to.key(),
        authority: ctx.accounts.authority.key(),
        amount,
    });

    Ok(())
}
