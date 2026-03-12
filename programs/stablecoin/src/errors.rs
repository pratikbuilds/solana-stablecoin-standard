use anchor_lang::prelude::*;

#[error_code]
pub enum StablecoinError {
    #[msg("Invalid stablecoin name")]
    InvalidName,
    #[msg("Invalid stablecoin symbol")]
    InvalidSymbol,
    #[msg("Invalid metadata URI")]
    InvalidUri,
    #[msg("Invalid decimal precision")]
    InvalidDecimals,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Transfer hook requires permanent delegate")]
    InvalidPresetConfiguration,
    #[msg("Compliance features are not enabled for this mint")]
    ComplianceNotEnabled,
    #[msg("Permanent delegate is not enabled for this mint")]
    PermanentDelegateNotEnabled,
    #[msg("Transfer hook is not enabled for this mint")]
    TransferHookNotEnabled,
    #[msg("Only the master authority can perform this action")]
    NotMasterAuthority,
    #[msg("Caller is not an active minter for this mint")]
    NotActiveMinter,
    #[msg("Caller is not allowed to burn")]
    NotBurner,
    #[msg("Caller is not allowed to pause or freeze")]
    NotPauser,
    #[msg("Caller is not allowed to manage the blacklist")]
    NotBlacklister,
    #[msg("Caller is not allowed to seize tokens")]
    NotSeizer,
    #[msg("Mint quota exceeded")]
    MintQuotaExceeded,
    #[msg("This mint is currently paused")]
    StablecoinPaused,
    #[msg("Blacklist entry already exists")]
    AlreadyBlacklisted,
    #[msg("Blacklist entry does not exist")]
    NotBlacklisted,
    #[msg("Target token account must be frozen before seizure")]
    TargetAccountNotFrozen,
    #[msg("Destination treasury account must be owned by the current authority")]
    InvalidTreasuryAccount,
    #[msg("Source token account owner does not match the blacklist entry")]
    InvalidBlacklistTarget,
    #[msg("Mint account does not match configuration")]
    InvalidMint,
    #[msg("Transfer hook program account is missing")]
    MissingTransferHookProgram,
    #[msg("Extra account meta list account is missing")]
    MissingExtraAccountMetaList,
    #[msg("Provided transfer hook program does not match the expected program")]
    InvalidTransferHookProgram,
    #[msg("Math overflow")]
    Overflow,
}
