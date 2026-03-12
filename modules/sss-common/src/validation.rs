use anchor_lang::prelude::*;

use crate::{MAX_NAME_LEN, MAX_SYMBOL_LEN, MAX_URI_LEN};

pub fn validate_metadata(name: &str, symbol: &str, uri: &str) -> Result<()> {
    require!(
        !name.is_empty() && name.len() <= MAX_NAME_LEN,
        ErrorCode::ConstraintRaw
    );
    require!(
        !symbol.is_empty() && symbol.len() <= MAX_SYMBOL_LEN,
        ErrorCode::ConstraintRaw
    );
    require!(uri.len() <= MAX_URI_LEN, ErrorCode::ConstraintRaw);
    Ok(())
}

pub fn validate_decimals(decimals: u8) -> Result<()> {
    require!(decimals <= 18, ErrorCode::ConstraintRaw);
    Ok(())
}
