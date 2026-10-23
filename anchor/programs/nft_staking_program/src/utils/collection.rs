use anchor_lang::prelude::*;
use mpl_core::{
    accounts::BaseCollectionV1,
    fetch_plugin,
    instructions::UpdateCollectionPluginV1CpiBuilder,
    types::{Attribute, Attributes, Plugin, PluginType},
};

use crate::{StakingError, TOTAL_STAKED_KEY};

/// Reads and updates the collection's `total_staked` attribute.
pub fn update_total_staked<'a>(
    core: &AccountInfo<'a>,
    collection: &AccountInfo<'a>,
    payer: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    seeds: &[&[u8]],
    delta: i64,
) -> Result<()> {
    let mut attributes =
        match fetch_plugin::<BaseCollectionV1, Attributes>(collection, PluginType::Attributes) {
            Ok((_, attrs, _)) => attrs.attribute_list,
            Err(_) => vec![],
        };

    let current: i64 = attributes
        .iter()
        .find(|attribute| attribute.key == TOTAL_STAKED_KEY)
        .and_then(|attribute| attribute.value.parse().ok())
        .unwrap_or(0);
    let next = current
        .checked_add(delta)
        .ok_or(StakingError::MathOverflow)?;
    require!(next >= 0, StakingError::MathOverflow);

    match attributes
        .iter_mut()
        .find(|attribute| attribute.key == TOTAL_STAKED_KEY)
    {
        Some(attribute) => attribute.value = next.to_string(),
        None => attributes.push(Attribute {
            key: TOTAL_STAKED_KEY.to_string(),
            value: next.to_string(),
        }),
    }

    UpdateCollectionPluginV1CpiBuilder::new(core)
        .collection(collection)
        .payer(payer)
        .authority(Some(authority))
        .system_program(system_program)
        .plugin(Plugin::Attributes(Attributes {
            attribute_list: attributes,
        }))
        .invoke_signed(&[seeds])?;

    Ok(())
}
