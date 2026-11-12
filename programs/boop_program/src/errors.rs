use anchor_lang::prelude::*;

#[error_code]
pub enum BoopError {
    #[msg("Unauthorized signer or account")]
    Unauthorized,
    #[msg("Feature is disabled for new activity")]
    Disabled,
    #[msg("Invalid input")]
    InvalidInput,
    #[msg("Unsupported mint or decimals")]
    InvalidMint,
    #[msg("Insufficient tracked balance")]
    InsufficientBalance,
    #[msg("Arithmetic overflow")]
    Math,
    #[msg("Reviewed price or fee changed")]
    PriceChanged,
    #[msg("Cancel the listing before transferring")]
    Listed,
    #[msg("Invalid or reserved tag")]
    InvalidTag,
    #[msg("Withdrawal approval expired or was already used")]
    InvalidApproval,
    #[msg("Daily withdrawal limit exceeded")]
    DailyLimit,
}
