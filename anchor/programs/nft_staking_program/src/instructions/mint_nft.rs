use anchor_lang::prelude::*;
use mpl_core::{instructions::CreateV2CpiBuilder, types::DataState};

use crate::MintNft;

pub fn handler(ctx: Context<MintNft>, name: String, uri: String) -> Result<()> {
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
    let admin = ctx.accounts.admin.to_account_info();
    let recipient = ctx.accounts.recipient.to_account_info();
    let system_program = ctx.accounts.system_program.to_account_info();

    CreateV2CpiBuilder::new(&core)
        .asset(&asset)
        .collection(Some(&collection))
        .authority(Some(&authority))
        .payer(&admin)
        .owner(Some(&recipient))
        .system_program(&system_program)
        .data_state(DataState::AccountState)
        .name(name)
        .uri(uri)
        .invoke_signed(&[seeds])?;

    Ok(())
}
