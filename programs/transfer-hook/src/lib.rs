#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("FETyQwfXmjCDh2UZTDTSrsu5XZJuT6abE39ox7st6sBn");

#[program]
pub mod transfer_hook {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.config.bump = ctx.bumps.config;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = payer, space = 8 + HookConfig::INIT_SPACE, seeds = [b"hook-config"], bump)]
    pub config: Account<'info, HookConfig>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct HookConfig {
    pub bump: u8,
}
