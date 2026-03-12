use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct RoleConfig {
    pub mint: Pubkey,
    pub master_authority: Pubkey,
    pub pauser: Pubkey,
    pub burner: Pubkey,
    pub blacklister: Pubkey,
    pub seizer: Pubkey,
    pub bump: u8,
}

impl RoleConfig {
    pub fn is_master(&self, key: &Pubkey) -> bool {
        self.master_authority == *key
    }

    pub fn is_pauser(&self, key: &Pubkey) -> bool {
        self.is_master(key) || self.pauser == *key
    }

    pub fn is_burner(&self, key: &Pubkey) -> bool {
        self.is_master(key) || self.burner == *key
    }

    pub fn is_blacklister(&self, key: &Pubkey) -> bool {
        self.is_master(key) || self.blacklister == *key
    }

    pub fn is_seizer(&self, key: &Pubkey) -> bool {
        self.is_master(key) || self.seizer == *key
    }
}
