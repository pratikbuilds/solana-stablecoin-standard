use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct BlacklistEntry {
    pub mint: Pubkey,
    pub wallet: Pubkey,
    #[max_len(128)]
    pub reason: String,
    pub blacklisted_by: Pubkey,
    pub blacklisted_at: i64,
    pub bump: u8,
}
