use anchor_lang::prelude::*;
use mpl_core::{
    accounts::BaseAssetV1,
    instructions::AddPluginV1CpiBuilder,
    types::{BurnDelegate, FreezeDelegate, Plugin, PluginAuthority, UpdateAuthority},
};

use crate::{utils::update_total_staked, Stake, StakingError};

pub fn handler(ctx: Context<Stake>) -> Result<()> {
    let collection_key = ctx.accounts.collection.key();
    let seeds: &[&[u8]] = &[
        b"authority",
        collection_key.as_ref(),
        &[ctx.bumps.authority],
    ];

    let core = ctx.accounts.core_program.to_account_info();
    let asset = ctx.accounts.asset.to_account_info();
    let collection = ctx.accounts.collection.to_account_info();
    let authority = ctx.accounts.authority.to_account_info();
    let owner = ctx.accounts.owner.to_account_info();
    let system_program = ctx.accounts.system_program.to_account_info();

    let base = BaseAssetV1::from_bytes(&asset.try_borrow_data()?)
        .map_err(|_| error!(StakingError::InvalidAsset))?;
    require_keys_eq!(base.owner, owner.key(), StakingError::NotOwner);
    require!(
        base.update_authority == UpdateAuthority::Collection(collection_key),
        StakingError::InvalidCollection
    );

    let delegate = PluginAuthority::Address {
        address: authority.key(),
    };

    AddPluginV1CpiBuilder::new(&core)
        .asset(&asset)
        .collection(Some(&collection))
        .payer(&owner)
        .authority(Some(&owner))
        .system_program(&system_program)
        .plugin(Plugin::BurnDelegate(BurnDelegate {}))
        .init_authority(delegate.clone())
        .invoke()?;

    AddPluginV1CpiBuilder::new(&core)
        .asset(&asset)
        .collection(Some(&collection))
        .payer(&owner)
        .authority(Some(&owner))
        .system_program(&system_program)
        .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
        .init_authority(delegate)
        .invoke()?;

    update_total_staked(
        &core,
        &collection,
        &owner,
        &authority,
        &system_program,
        seeds,
        1,
    )?;

    let now = Clock::get()?.unix_timestamp;
    let record = &mut ctx.accounts.stake_record;
    record.owner = owner.key();
    record.asset = asset.key();
    record.staked_at = now;
    record.last_claim = now;

    Ok(())
}
