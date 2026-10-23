use anchor_lang::prelude::*;

#[error_code]
pub enum StakingError {
    #[msg("Invalid Core asset")]
    InvalidAsset,
    #[msg("Signer does not own this asset")]
    NotOwner,
    #[msg("Asset is not part of this collection")]
    InvalidCollection,
    #[msg("No rewards to claim yet")]
    NothingToClaim,
    #[msg("Math overflow")]
    MathOverflow,
}
