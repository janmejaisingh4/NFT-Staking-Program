#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod state;
pub mod utils;

pub use errors::StakingError;
pub use instructions::accounts::*;
pub use state::{Config, StakeRecord};

pub const TOTAL_STAKED_KEY: &str = "total_staked";

declare_id!("HBYJKygWr7gyRa4N3mCdEziW2KAQB9y9TMPcEaLESMoR");

#[program]
pub mod nft_staking_core {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        name: String,
        uri: String,
        reward_per_second: u64,
        burn_bonus: u64,
    ) -> Result<()> {
        instructions::initialize::handler(ctx, name, uri, reward_per_second, burn_bonus)
    }

    pub fn mint_nft(ctx: Context<MintNft>, name: String, uri: String) -> Result<()> {
        instructions::mint_nft::handler(ctx, name, uri)
    }

    pub fn stake(ctx: Context<Stake>) -> Result<()> {
        instructions::stake::handler(ctx)
    }

    pub fn claim_rewards(ctx: Context<Claim>) -> Result<()> {
        instructions::claim_rewards::handler(ctx)
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        instructions::unstake::handler(ctx)
    }

    pub fn burn_staked_nft(ctx: Context<Unstake>) -> Result<()> {
        instructions::burn_staked_nft::handler(ctx)
    }
}
