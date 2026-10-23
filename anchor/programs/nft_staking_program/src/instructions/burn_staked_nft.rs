use anchor_lang::prelude::*;
use mpl_core::{
    instructions::{BurnV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{FreezeDelegate, Plugin},
};

use crate::{
    utils::{mint_rewards, pending_rewards, update_total_staked},
    StakingError, Unstake,
};

pub fn handler(ctx: Context<Unstake>) -> Result<()> {
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
    let total = pending
        .checked_add(ctx.accounts.config.burn_bonus)
        .ok_or(StakingError::MathOverflow)?;

    let core = ctx.accounts.core_program.to_account_info();
    let asset = ctx.accounts.asset.to_account_info();
    let collection = ctx.accounts.collection.to_account_info();
    let authority = ctx.accounts.authority.to_account_info();
    let owner = ctx.accounts.owner.to_account_info();
    let system_program = ctx.accounts.system_program.to_account_info();

    UpdatePluginV1CpiBuilder::new(&core)
        .asset(&asset)
        .collection(Some(&collection))
        .payer(&owner)
        .authority(Some(&authority))
        .system_program(&system_program)
        .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
        .invoke_signed(&[seeds])?;

    BurnV1CpiBuilder::new(&core)
        .asset(&asset)
        .collection(Some(&collection))
        .payer(&owner)
        .authority(Some(&authority))
        .system_program(Some(&system_program))
        .invoke_signed(&[seeds])?;

    update_total_staked(
        &core,
        &collection,
        &owner,
        &authority,
        &system_program,
        seeds,
        -1,
    )?;

    mint_rewards(
        &ctx.accounts.token_program.to_account_info(),
        &ctx.accounts.reward_mint.to_account_info(),
        &ctx.accounts.owner_ata.to_account_info(),
        &authority,
        seeds,
        total,
    )
}
