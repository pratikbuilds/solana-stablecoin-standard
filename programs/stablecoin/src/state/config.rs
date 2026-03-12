use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct StablecoinConfig {
    pub mint: Pubkey,
    pub authority: Pubkey,
    #[max_len(32)]
    pub name: String,
    #[max_len(10)]
    pub symbol: String,
    #[max_len(200)]
    pub uri: String,
    pub decimals: u8,
    pub enable_permanent_delegate: bool,
    pub enable_transfer_hook: bool,
    pub default_account_frozen: bool,
    pub paused: bool,
    pub total_minted: u64,
    pub total_burned: u64,
    pub created_at: i64,
    pub last_changed_by: Pubkey,
    pub last_changed_at: i64,
    pub bump: u8,
}

impl StablecoinConfig {
    pub fn is_sss2(&self) -> bool {
        self.enable_permanent_delegate && self.enable_transfer_hook
    }
}
