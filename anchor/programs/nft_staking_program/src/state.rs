use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    pub collection: Pubkey,
    pub reward_mint: Pubkey,
    pub reward_per_second: u64,
    pub burn_bonus: u64,
}

#[account]
#[derive(InitSpace)]
pub struct StakeRecord {
    pub owner: Pubkey,
    pub asset: Pubkey,
    pub staked_at: i64,
    pub last_claim: i64,
}
