use anchor_lang::prelude::*;
use mpl_core::{
    instructions::CreateCollectionV2CpiBuilder,
    types::{Attribute, Attributes, Plugin, PluginAuthorityPair},
};

use crate::{Initialize, TOTAL_STAKED_KEY};

pub fn handler(
    ctx: Context<Initialize>,
    name: String,
    uri: String,
    reward_per_second: u64,
    burn_bonus: u64,
) -> Result<()> {
    let cfg = &mut ctx.accounts.config;
    cfg.admin = ctx.accounts.admin.key();
    cfg.collection = ctx.accounts.collection.key();
    cfg.reward_mint = ctx.accounts.reward_mint.key();
    cfg.reward_per_second = reward_per_second;
    cfg.burn_bonus = burn_bonus;

    let core = ctx.accounts.core_program.to_account_info();
    let collection = ctx.accounts.collection.to_account_info();
    let authority = ctx.accounts.authority.to_account_info();
    let admin = ctx.accounts.admin.to_account_info();
    let system_program = ctx.accounts.system_program.to_account_info();

    CreateCollectionV2CpiBuilder::new(&core)
        .collection(&collection)
        .update_authority(Some(&authority))
        .payer(&admin)
        .system_program(&system_program)
        .name(name)
        .uri(uri)
        .plugins(vec![PluginAuthorityPair {
            plugin: Plugin::Attributes(Attributes {
                attribute_list: vec![Attribute {
                    key: TOTAL_STAKED_KEY.to_string(),
                    value: "0".to_string(),
                }],
            }),
            authority: None,
        }])
        .invoke()?;

    Ok(())
}
