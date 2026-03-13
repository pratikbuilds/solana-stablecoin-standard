use anchor_lang::prelude::*;

use sss_common::{SEED_CONFIG, SEED_ROLES};

use crate::errors::StablecoinError;
use crate::events::PauseChanged;
use crate::state::{RoleConfig, StablecoinConfig};

#[event_cpi]
#[derive(Accounts)]
pub struct PauseOps<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_CONFIG, config.mint.as_ref()],
        bump = config.bump
    )]
    pub config: Account<'info, StablecoinConfig>,
    #[account(
        seeds = [SEED_ROLES, config.mint.as_ref()],
        bump = role_config.bump
    )]
    pub role_config: Account<'info, RoleConfig>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct UnpauseOps<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_CONFIG, config.mint.as_ref()],
        bump = config.bump
    )]
    pub config: Account<'info, StablecoinConfig>,
    #[account(
        seeds = [SEED_ROLES, config.mint.as_ref()],
        bump = role_config.bump
    )]
    pub role_config: Account<'info, RoleConfig>,
}

pub fn pause_handler(ctx: Context<PauseOps>) -> Result<()> {
    require!(
        ctx.accounts
            .role_config
            .is_pauser(&ctx.accounts.authority.key()),
        StablecoinError::NotPauser
    );

    ctx.accounts.config.paused = true;
    ctx.accounts.config.last_changed_by = ctx.accounts.authority.key();
    ctx.accounts.config.last_changed_at = Clock::get()?.unix_timestamp;

    emit_cpi!(PauseChanged {
        mint: ctx.accounts.config.mint,
        paused: true,
        authority: ctx.accounts.authority.key(),
    });

    Ok(())
}

pub fn unpause_handler(ctx: Context<UnpauseOps>) -> Result<()> {
    require!(
        ctx.accounts
            .role_config
            .is_pauser(&ctx.accounts.authority.key()),
        StablecoinError::NotPauser
    );

    ctx.accounts.config.paused = false;
    ctx.accounts.config.last_changed_by = ctx.accounts.authority.key();
    ctx.accounts.config.last_changed_at = Clock::get()?.unix_timestamp;

    emit_cpi!(PauseChanged {
        mint: ctx.accounts.config.mint,
        paused: false,
        authority: ctx.accounts.authority.key(),
    });

    Ok(())
}
