use anchor_lang::prelude::*;

use crate::{
    utils::{mint_rewards, pending_rewards},
    Claim, StakingError,
};

pub fn handler(ctx: Context<Claim>) -> Result<()> {
    let collection_key = ctx.accounts.collection.key();
    let seeds: &[&[u8]] = &[
        b"authority",
        collection_key.as_ref(),
        &[ctx.bumps.authority],
    ];

    let now = Clock::get()?.unix_timestamp;
    let pending = pending_rewards(
        &ctx.accounts.stake_record,
        ctx.accounts.config.reward_per_second,
        now,
    )?;
    require!(pending > 0, StakingError::NothingToClaim);

    ctx.accounts.stake_record.last_claim = now;

    mint_rewards(
        &ctx.accounts.token_program.to_account_info(),
        &ctx.accounts.reward_mint.to_account_info(),
        &ctx.accounts.owner_ata.to_account_info(),
        &ctx.accounts.authority.to_account_info(),
        seeds,
        pending,
    )
}
