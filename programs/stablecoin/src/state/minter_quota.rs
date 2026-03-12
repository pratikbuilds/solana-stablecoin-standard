use anchor_lang::prelude::*;

use crate::errors::StablecoinError;

#[account]
#[derive(InitSpace)]
pub struct MinterQuota {
    pub mint: Pubkey,
    pub minter: Pubkey,
    pub quota: u64,
    pub minted: u64,
    pub active: bool,
    pub created_at: i64,
    pub bump: u8,
}

impl MinterQuota {
    pub fn can_mint(&self, amount: u64) -> bool {
        self.active
            && self
                .minted
                .checked_add(amount)
                .map(|total| total <= self.quota)
                .unwrap_or(false)
    }

    pub fn record_mint(&mut self, amount: u64) -> Result<()> {
        self.minted = self
            .minted
            .checked_add(amount)
            .ok_or(error!(StablecoinError::Overflow))?;
        Ok(())
    }
}
