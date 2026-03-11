#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("78YRTUZwe3Rx56tFV77MiQVrrMTLtkYDSLAPEAdYRWyf");

#[program]
pub mod stablecoin {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.state.bump = ctx.bumps.state;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = payer, space = 8 + StablecoinState::INIT_SPACE, seeds = [b"state"], bump)]
    pub state: Account<'info, StablecoinState>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct StablecoinState {
    pub bump: u8,
}
