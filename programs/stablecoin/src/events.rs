use anchor_lang::prelude::*;

#[event]
pub struct StablecoinInitialized {
    pub mint: Pubkey,
    pub authority: Pubkey,
    pub preset: String,
}

#[event]
pub struct MinterUpdated {
    pub mint: Pubkey,
    pub minter: Pubkey,
    pub quota: u64,
    pub active: bool,
}

#[event]
pub struct RolesUpdated {
    pub mint: Pubkey,
    pub authority: Pubkey,
}

#[event]
pub struct AuthorityTransferred {
    pub mint: Pubkey,
    pub old_authority: Pubkey,
    pub new_authority: Pubkey,
}

#[event]
pub struct TokensMinted {
    pub mint: Pubkey,
    pub to: Pubkey,
    pub authority: Pubkey,
    pub amount: u64,
}

#[event]
pub struct TokensBurned {
    pub mint: Pubkey,
    pub from: Pubkey,
    pub authority: Pubkey,
    pub amount: u64,
}

#[event]
pub struct AccountFrozen {
    pub mint: Pubkey,
    pub account: Pubkey,
    pub authority: Pubkey,
}

#[event]
pub struct AccountThawed {
    pub mint: Pubkey,
    pub account: Pubkey,
    pub authority: Pubkey,
}

#[event]
pub struct PauseChanged {
    pub mint: Pubkey,
    pub paused: bool,
    pub authority: Pubkey,
}

#[event]
pub struct AddressBlacklisted {
    pub mint: Pubkey,
    pub wallet: Pubkey,
    pub authority: Pubkey,
    pub reason: String,
}

#[event]
pub struct AddressUnblacklisted {
    pub mint: Pubkey,
    pub wallet: Pubkey,
    pub authority: Pubkey,
}

#[event]
pub struct TokensSeized {
    pub mint: Pubkey,
    pub from: Pubkey,
    pub to: Pubkey,
    pub authority: Pubkey,
    pub amount: u64,
}
