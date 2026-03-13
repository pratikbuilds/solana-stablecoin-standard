use anchor_lang::prelude::*;
use anchor_spl::token_interface::{burn, Burn, Mint, TokenAccount, TokenInterface};

use sss_common::{SEED_CONFIG, SEED_ROLES};

use crate::errors::StablecoinError;
use crate::events::TokensBurned;
use crate::state::{RoleConfig, StablecoinConfig};

#[event_cpi]
#[derive(Accounts)]
pub struct BurnTokens<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
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
    pub from: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn burn_handler(ctx: Context<BurnTokens>, amount: u64) -> Result<()> {
    require!(amount > 0, StablecoinError::ZeroAmount);
    require!(
        !ctx.accounts.config.paused,
        StablecoinError::StablecoinPaused
    );
    require!(
        ctx.accounts
            .role_config
            .is_burner(&ctx.accounts.authority.key()),
        StablecoinError::NotBurner
    );
    require!(
        ctx.accounts.from.owner == ctx.accounts.authority.key(),
        StablecoinError::NotBurner
    );

    burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.mint.to_account_info(),
                from: ctx.accounts.from.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        ),
        amount,
    )?;

    ctx.accounts.config.total_burned = ctx
        .accounts
        .config
        .total_burned
        .checked_add(amount)
        .ok_or(error!(StablecoinError::Overflow))?;

    emit_cpi!(TokensBurned {
        mint: ctx.accounts.mint.key(),
        from: ctx.accounts.from.key(),
        authority: ctx.accounts.authority.key(),
        amount,
    });

    Ok(())
}
