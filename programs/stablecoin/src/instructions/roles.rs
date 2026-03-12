use anchor_lang::prelude::*;

use sss_common::{SEED_CONFIG, SEED_MINTER, SEED_ROLES};

use crate::errors::StablecoinError;
use crate::events::{AuthorityTransferred, MinterUpdated, RolesUpdated};
use crate::state::{MinterQuota, RoleConfig, StablecoinConfig};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UpdateMinterParams {
    pub minter: Pubkey,
    pub quota: u64,
    pub active: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct UpdateRolesParams {
    pub pauser: Option<Pubkey>,
    pub burner: Option<Pubkey>,
    pub blacklister: Option<Pubkey>,
    pub seizer: Option<Pubkey>,
}

#[derive(Accounts)]
pub struct UpdateMinter<'info> {
    #[account(mut)]
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
    /// CHECK: Mint pubkey is only used for PDA derivation and emitted in events.
    pub mint: UncheckedAccount<'info>,
    /// CHECK: Used only for PDA derivation.
    pub minter: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = authority,
        space = 8 + MinterQuota::INIT_SPACE,
        seeds = [SEED_MINTER, mint.key().as_ref(), minter.key().as_ref()],
        bump
    )]
    pub minter_quota: Account<'info, MinterQuota>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateRoles<'info> {
    pub authority: Signer<'info>,
    #[account(
        seeds = [SEED_CONFIG, config.mint.as_ref()],
        bump = config.bump
    )]
    pub config: Account<'info, StablecoinConfig>,
    #[account(
        mut,
        seeds = [SEED_ROLES, config.mint.as_ref()],
        bump = role_config.bump
    )]
    pub role_config: Account<'info, RoleConfig>,
}

#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_CONFIG, config.mint.as_ref()],
        bump = config.bump
    )]
    pub config: Account<'info, StablecoinConfig>,
    #[account(
        mut,
        seeds = [SEED_ROLES, config.mint.as_ref()],
        bump = role_config.bump
    )]
    pub role_config: Account<'info, RoleConfig>,
}

pub fn update_minter_handler(ctx: Context<UpdateMinter>, params: UpdateMinterParams) -> Result<()> {
    require!(
        ctx.accounts
            .role_config
            .is_master(&ctx.accounts.authority.key()),
        StablecoinError::NotMasterAuthority
    );
    require!(
        params.minter == ctx.accounts.minter.key(),
        StablecoinError::NotMasterAuthority
    );

    let now = Clock::get()?.unix_timestamp;
    let minter_quota = &mut ctx.accounts.minter_quota;
    if minter_quota.created_at == 0 {
        minter_quota.created_at = now;
        minter_quota.mint = ctx.accounts.mint.key();
        minter_quota.minter = params.minter;
        minter_quota.minted = 0;
        minter_quota.bump = ctx.bumps.minter_quota;
    }

    minter_quota.quota = params.quota;
    minter_quota.active = params.active;

    emit!(MinterUpdated {
        mint: ctx.accounts.mint.key(),
        minter: params.minter,
        quota: params.quota,
        active: params.active,
    });

    Ok(())
}

pub fn update_roles_handler(ctx: Context<UpdateRoles>, params: UpdateRolesParams) -> Result<()> {
    require!(
        ctx.accounts
            .role_config
            .is_master(&ctx.accounts.authority.key()),
        StablecoinError::NotMasterAuthority
    );

    if params.blacklister.is_some() || params.seizer.is_some() {
        require!(
            ctx.accounts.config.is_sss2(),
            StablecoinError::ComplianceNotEnabled
        );
    }

    let role_config = &mut ctx.accounts.role_config;
    if let Some(pauser) = params.pauser {
        role_config.pauser = pauser;
    }
    if let Some(burner) = params.burner {
        role_config.burner = burner;
    }
    if let Some(blacklister) = params.blacklister {
        role_config.blacklister = blacklister;
    }
    if let Some(seizer) = params.seizer {
        role_config.seizer = seizer;
    }

    emit!(RolesUpdated {
        mint: ctx.accounts.config.mint,
        authority: ctx.accounts.authority.key(),
    });

    Ok(())
}

pub fn transfer_authority_handler(
    ctx: Context<TransferAuthority>,
    new_authority: Pubkey,
) -> Result<()> {
    require!(
        ctx.accounts
            .role_config
            .is_master(&ctx.accounts.authority.key()),
        StablecoinError::NotMasterAuthority
    );

    let old_authority = ctx.accounts.config.authority;
    ctx.accounts.config.authority = new_authority;
    ctx.accounts.role_config.master_authority = new_authority;

    emit!(AuthorityTransferred {
        mint: ctx.accounts.config.mint,
        old_authority,
        new_authority,
    });

    Ok(())
}
