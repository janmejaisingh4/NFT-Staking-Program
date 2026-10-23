use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use mpl_core::ID as CORE_ID;

use crate::{Config, StakeRecord};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    /// New collection keypair (signs the tx)
    #[account(mut)]
    pub collection: Signer<'info>,
    /// CHECK: PDA, collection update authority / delegate / mint authority
    #[account(seeds = [b"authority", collection.key().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,
    #[account(init, payer = admin, space = 8 + Config::INIT_SPACE,
        seeds = [b"config", collection.key().as_ref()], bump)]
    pub config: Account<'info, Config>,
    #[account(init, payer = admin, seeds = [b"rewards", collection.key().as_ref()], bump,
        mint::decimals = 6, mint::authority = authority)]
    pub reward_mint: Account<'info, Mint>,
    /// CHECK: address-checked
    #[account(address = CORE_ID)]
    pub core_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintNft<'info> {
    #[account(mut, address = config.admin)]
    pub admin: Signer<'info>,
    #[account(mut)]
    pub asset: Signer<'info>,
    /// CHECK: validated by Core
    #[account(mut, address = config.collection)]
    pub collection: UncheckedAccount<'info>,
    /// CHECK: PDA
    #[account(seeds = [b"authority", collection.key().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,
    #[account(seeds = [b"config", collection.key().as_ref()], bump)]
    pub config: Account<'info, Config>,
    /// CHECK: any wallet
    pub recipient: UncheckedAccount<'info>,
    /// CHECK: address-checked
    #[account(address = CORE_ID)]
    pub core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: validated in handler
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,
    /// CHECK: address-checked against config
    #[account(mut, address = config.collection)]
    pub collection: UncheckedAccount<'info>,
    /// CHECK: PDA
    #[account(seeds = [b"authority", collection.key().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,
    #[account(seeds = [b"config", collection.key().as_ref()], bump)]
    pub config: Account<'info, Config>,
    #[account(init, payer = owner, space = 8 + StakeRecord::INIT_SPACE,
        seeds = [b"stake", asset.key().as_ref()], bump)]
    pub stake_record: Account<'info, StakeRecord>,
    /// CHECK: address-checked
    #[account(address = CORE_ID)]
    pub core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: only used for the stake_record seed
    pub asset: UncheckedAccount<'info>,
    /// CHECK: address-checked
    #[account(address = config.collection)]
    pub collection: UncheckedAccount<'info>,
    /// CHECK: PDA mint authority
    #[account(seeds = [b"authority", collection.key().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,
    #[account(seeds = [b"config", collection.key().as_ref()], bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"stake", asset.key().as_ref()], bump, has_one = owner)]
    pub stake_record: Account<'info, StakeRecord>,
    #[account(mut, address = config.reward_mint)]
    pub reward_mint: Account<'info, Mint>,
    #[account(init_if_needed, payer = owner,
        associated_token::mint = reward_mint, associated_token::authority = owner)]
    pub owner_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

/// Shared by `unstake` and `burn_staked_nft`.
#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: validated by Core + stake_record seeds
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,
    /// CHECK: address-checked
    #[account(mut, address = config.collection)]
    pub collection: UncheckedAccount<'info>,
    /// CHECK: PDA
    #[account(seeds = [b"authority", collection.key().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,
    #[account(seeds = [b"config", collection.key().as_ref()], bump)]
    pub config: Account<'info, Config>,
    #[account(mut, close = owner, seeds = [b"stake", asset.key().as_ref()], bump, has_one = owner)]
    pub stake_record: Account<'info, StakeRecord>,
    #[account(mut, address = config.reward_mint)]
    pub reward_mint: Account<'info, Mint>,
    #[account(init_if_needed, payer = owner,
        associated_token::mint = reward_mint, associated_token::authority = owner)]
    pub owner_ata: Account<'info, TokenAccount>,
    /// CHECK: address-checked
    #[account(address = CORE_ID)]
    pub core_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
