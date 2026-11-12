use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Only the vault admin can do this")]
    Unauthorized,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Deposit is too small: zero shares")]
    ZeroShares,
}
