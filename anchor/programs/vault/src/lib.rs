use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
};
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, MintTo, Token, TokenAccount},
};

const CORE_PROGRAM_ID: Pubkey = pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");

#[cfg(test)]
mod staking_tests;
#[cfg(test)]
mod tests;

declare_id!("7tGGvLyrnSdFr5o8Qcfbw2spARwMfFCf7sv8bMZ3k9mv");

#[program]
pub mod vault {
    use super::*;

    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        rewards_per_second: u64,
        burn_bonus: u64,
    ) -> Result<()> {
        require!(
            rewards_per_second > 0 && burn_bonus > 0,
            VaultError::InvalidAmount
        );

        let pool = &mut ctx.accounts.pool;
        pool.admin = ctx.accounts.admin.key();
        pool.collection = ctx.accounts.collection.key();
        pool.reward_mint = ctx.accounts.reward_mint.key();
        pool.rewards_per_second = rewards_per_second;
        pool.burn_bonus = burn_bonus;
        pool.total_staked = 0;
        pool.authority_bump = ctx.bumps.authority;
        pool.bump = ctx.bumps.pool;

        invoke_core_add_collection_plugin(
            &ctx.accounts.core_program,
            &ctx.accounts.collection,
            &ctx.accounts.admin,
            &ctx.accounts.admin,
            &ctx.accounts.system_program,
            encode_attributes_plugin(0),
            ctx.accounts.authority.key(),
        )?;

        Ok(())
    }

    pub fn stake_nft(ctx: Context<StakeNft>) -> Result<()> {
        let authority_bump = [ctx.accounts.pool.authority_bump];
        let collection_key = ctx.accounts.collection.key();
        let authority_seeds: &[&[u8]] =
            &[b"pool-authority", collection_key.as_ref(), &authority_bump];

        invoke_core_add_asset_plugin(
            &ctx.accounts.core_program,
            &ctx.accounts.asset,
            &ctx.accounts.collection,
            &ctx.accounts.user,
            &ctx.accounts.user,
            &ctx.accounts.system_program,
            vec![2],
            ctx.accounts.authority.key(),
        )?;
        invoke_core_add_asset_plugin(
            &ctx.accounts.core_program,
            &ctx.accounts.asset,
            &ctx.accounts.collection,
            &ctx.accounts.user,
            &ctx.accounts.user,
            &ctx.accounts.system_program,
            vec![1, 1],
            ctx.accounts.authority.key(),
        )?;

        let total_staked = ctx
            .accounts
            .pool
            .total_staked
            .checked_add(1)
            .ok_or(VaultError::RewardCalculationOverflow)?;
        update_collection_count(
            &ctx.accounts.core_program,
            &ctx.accounts.collection,
            &ctx.accounts.user,
            &ctx.accounts.authority,
            &ctx.accounts.system_program,
            total_staked,
            authority_seeds,
        )?;

        let now = Clock::get()?.unix_timestamp;
        let stake = &mut ctx.accounts.stake_record;
        stake.owner = ctx.accounts.user.key();
        stake.asset = ctx.accounts.asset.key();
        stake.collection = ctx.accounts.collection.key();
        stake.staked_at = now;
        stake.last_claimed_at = now;
        stake.bump = ctx.bumps.stake_record;
        ctx.accounts.pool.total_staked = total_staked;

        Ok(())
    }

    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let amount = accrued_rewards(&ctx.accounts.pool, &ctx.accounts.stake_record, now)?;
        require!(amount > 0, VaultError::NoRewardsAvailable);

        let authority_bump = [ctx.accounts.pool.authority_bump];
        let authority_seeds: &[&[u8]] = &[
            b"pool-authority",
            ctx.accounts.pool.collection.as_ref(),
            &authority_bump,
        ];
        let signer_seeds: &[&[&[u8]]] = &[authority_seeds];
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.reward_mint.to_account_info(),
                    to: ctx.accounts.user_reward_account.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;
        ctx.accounts.stake_record.last_claimed_at = now;

        Ok(())
    }

    pub fn unstake_nft(ctx: Context<UnstakeNft>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let amount = accrued_rewards(&ctx.accounts.pool, &ctx.accounts.stake_record, now)?;
        let authority_bump = [ctx.accounts.pool.authority_bump];
        let authority_seeds: &[&[u8]] = &[
            b"pool-authority",
            ctx.accounts.pool.collection.as_ref(),
            &authority_bump,
        ];
        let signer_seeds: &[&[&[u8]]] = &[authority_seeds];

        if amount > 0 {
            token::mint_to(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    MintTo {
                        mint: ctx.accounts.reward_mint.to_account_info(),
                        to: ctx.accounts.user_reward_account.to_account_info(),
                        authority: ctx.accounts.authority.to_account_info(),
                    },
                    signer_seeds,
                ),
                amount,
            )?;
        }

        invoke_core_update_asset_plugin(
            &ctx.accounts.core_program,
            &ctx.accounts.asset,
            &ctx.accounts.collection,
            &ctx.accounts.user,
            &ctx.accounts.authority,
            &ctx.accounts.system_program,
            vec![1, 0],
            signer_seeds,
        )?;
        invoke_core_remove_asset_plugin(
            &ctx.accounts.core_program,
            &ctx.accounts.asset,
            &ctx.accounts.collection,
            &ctx.accounts.user,
            &ctx.accounts.authority,
            &ctx.accounts.system_program,
            1,
            signer_seeds,
        )?;
        invoke_core_remove_asset_plugin(
            &ctx.accounts.core_program,
            &ctx.accounts.asset,
            &ctx.accounts.collection,
            &ctx.accounts.user,
            &ctx.accounts.authority,
            &ctx.accounts.system_program,
            2,
            signer_seeds,
        )?;

        let total_staked = ctx
            .accounts
            .pool
            .total_staked
            .checked_sub(1)
            .ok_or(VaultError::NoStakedAssets)?;
        update_collection_count(
            &ctx.accounts.core_program,
            &ctx.accounts.collection,
            &ctx.accounts.user,
            &ctx.accounts.authority,
            &ctx.accounts.system_program,
            total_staked,
            authority_seeds,
        )?;
        ctx.accounts.pool.total_staked = total_staked;

        Ok(())
    }

    pub fn burn_staked_nft(ctx: Context<BurnStakedNft>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let accrued = accrued_rewards(&ctx.accounts.pool, &ctx.accounts.stake_record, now)?;
        let reward_amount = accrued
            .checked_add(ctx.accounts.pool.burn_bonus)
            .ok_or(VaultError::RewardCalculationOverflow)?;
        let authority_bump = [ctx.accounts.pool.authority_bump];
        let authority_seeds: &[&[u8]] = &[
            b"pool-authority",
            ctx.accounts.pool.collection.as_ref(),
            &authority_bump,
        ];
        let signer_seeds: &[&[&[u8]]] = &[authority_seeds];

        invoke_core_burn_asset(
            &ctx.accounts.core_program,
            &ctx.accounts.asset,
            &ctx.accounts.collection,
            &ctx.accounts.user,
            &ctx.accounts.authority,
            &ctx.accounts.system_program,
            signer_seeds,
        )?;

        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.reward_mint.to_account_info(),
                    to: ctx.accounts.user_reward_account.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
                signer_seeds,
            ),
            reward_amount,
        )?;

        let total_staked = ctx
            .accounts
            .pool
            .total_staked
            .checked_sub(1)
            .ok_or(VaultError::NoStakedAssets)?;
        update_collection_count(
            &ctx.accounts.core_program,
            &ctx.accounts.collection,
            &ctx.accounts.user,
            &ctx.accounts.authority,
            &ctx.accounts.system_program,
            total_staked,
            authority_seeds,
        )?;
        ctx.accounts.pool.total_staked = total_staked;

        Ok(())
    }

    pub fn deposit(ctx: Context<VaultAction>, amount: u64) -> Result<()> {
        require!(
            ctx.accounts.vault.lamports() == 0,
            VaultError::VaultAlreadyExists
        );

        let rent = Rent::get()?.minimum_balance(0);
        require!(amount > rent, VaultError::InvalidAmount);

        transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.signer.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                },
            ),
            amount,
        )?;

        Ok(())
    }

    pub fn withdraw(ctx: Context<VaultAction>) -> Result<()> {
        require!(ctx.accounts.vault.lamports() > 0, VaultError::InvalidAmount);

        let bump = ctx.bumps.vault;
        let signer_key = ctx.accounts.signer.key();
        let signer_seeds: &[&[&[u8]]] = &[&[b"vault", signer_key.as_ref(), &[bump]]];

        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.signer.to_account_info(),
                },
                signer_seeds,
            ),
            ctx.accounts.vault.lamports(),
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct VaultAction<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", signer.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut, owner = CORE_PROGRAM_ID)]
    /// CHECK: Core validates this collection during the plugin CPI.
    pub collection: UncheckedAccount<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + StakingPool::INIT_SPACE,
        seeds = [b"pool", collection.key().as_ref()],
        bump,
    )]
    pub pool: Account<'info, StakingPool>,
    /// CHECK: PDA is used only as the reward mint and Core plugin authority.
    #[account(seeds = [b"pool-authority", collection.key().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = admin,
        mint::decimals = 9,
        mint::authority = authority,
    )]
    pub reward_mint: Account<'info, Mint>,
    /// CHECK: Constrained to the Metaplex Core program ID.
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct StakeNft<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, has_one = collection)]
    pub pool: Account<'info, StakingPool>,
    #[account(mut, address = pool.collection, owner = CORE_PROGRAM_ID)]
    /// CHECK: Core validates collection membership during the CPI.
    pub collection: UncheckedAccount<'info>,
    #[account(mut, owner = CORE_PROGRAM_ID)]
    /// CHECK: Core validates this asset and its owner during plugin CPIs.
    pub asset: UncheckedAccount<'info>,
    #[account(
        init,
        payer = user,
        space = 8 + StakeRecord::INIT_SPACE,
        seeds = [b"stake", asset.key().as_ref()],
        bump,
    )]
    pub stake_record: Account<'info, StakeRecord>,
    /// CHECK: PDA signs only the Core plugin and token mint CPIs.
    #[account(seeds = [b"pool-authority", collection.key().as_ref()], bump = pool.authority_bump)]
    pub authority: UncheckedAccount<'info>,
    /// CHECK: Constrained to the Metaplex Core program ID.
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(has_one = collection, has_one = reward_mint)]
    pub pool: Account<'info, StakingPool>,
    #[account(address = pool.collection)]
    /// CHECK: Pool stores the canonical collection address.
    pub collection: UncheckedAccount<'info>,
    #[account(mut, address = pool.reward_mint)]
    pub reward_mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [b"stake", asset.key().as_ref()],
        bump = stake_record.bump,
        constraint = stake_record.owner == user.key() @ VaultError::Unauthorized,
        constraint = stake_record.asset == asset.key() @ VaultError::InvalidStakeRecord,
        constraint = stake_record.collection == pool.collection @ VaultError::InvalidStakeRecord,
    )]
    pub stake_record: Account<'info, StakeRecord>,
    /// CHECK: Staking record binds this asset to its owner.
    pub asset: UncheckedAccount<'info>,
    /// CHECK: PDA controls the configured reward mint.
    #[account(seeds = [b"pool-authority", collection.key().as_ref()], bump = pool.authority_bump)]
    pub authority: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = reward_mint,
        associated_token::authority = user,
    )]
    pub user_reward_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UnstakeNft<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, has_one = collection, has_one = reward_mint)]
    pub pool: Account<'info, StakingPool>,
    #[account(mut, address = pool.collection, owner = CORE_PROGRAM_ID)]
    /// CHECK: Pool stores the canonical collection and Core validates the CPI.
    pub collection: UncheckedAccount<'info>,
    #[account(mut, owner = CORE_PROGRAM_ID)]
    /// CHECK: Stake record binds this Core asset to the user.
    pub asset: UncheckedAccount<'info>,
    #[account(
        mut,
        close = user,
        seeds = [b"stake", asset.key().as_ref()],
        bump = stake_record.bump,
        constraint = stake_record.owner == user.key() @ VaultError::Unauthorized,
        constraint = stake_record.asset == asset.key() @ VaultError::InvalidStakeRecord,
        constraint = stake_record.collection == pool.collection @ VaultError::InvalidStakeRecord,
    )]
    pub stake_record: Account<'info, StakeRecord>,
    #[account(mut, address = pool.reward_mint)]
    pub reward_mint: Account<'info, Mint>,
    /// CHECK: PDA controls the reward mint and FreezeDelegate plugin.
    #[account(seeds = [b"pool-authority", collection.key().as_ref()], bump = pool.authority_bump)]
    pub authority: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = reward_mint,
        associated_token::authority = user,
    )]
    pub user_reward_account: Account<'info, TokenAccount>,
    /// CHECK: Constrained to the Metaplex Core program ID.
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BurnStakedNft<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, has_one = collection, has_one = reward_mint)]
    pub pool: Account<'info, StakingPool>,
    #[account(mut, address = pool.collection, owner = CORE_PROGRAM_ID)]
    /// CHECK: Pool stores the canonical collection and Core validates the CPI.
    pub collection: UncheckedAccount<'info>,
    #[account(mut, owner = CORE_PROGRAM_ID)]
    /// CHECK: Stake record binds this Core asset to the user.
    pub asset: UncheckedAccount<'info>,
    #[account(
        mut,
        close = user,
        seeds = [b"stake", asset.key().as_ref()],
        bump = stake_record.bump,
        constraint = stake_record.owner == user.key() @ VaultError::Unauthorized,
        constraint = stake_record.asset == asset.key() @ VaultError::InvalidStakeRecord,
        constraint = stake_record.collection == pool.collection @ VaultError::InvalidStakeRecord,
    )]
    pub stake_record: Account<'info, StakeRecord>,
    #[account(mut, address = pool.reward_mint)]
    pub reward_mint: Account<'info, Mint>,
    /// CHECK: PDA controls the reward mint and BurnDelegate plugin.
    #[account(seeds = [b"pool-authority", collection.key().as_ref()], bump = pool.authority_bump)]
    pub authority: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = reward_mint,
        associated_token::authority = user,
    )]
    pub user_reward_account: Account<'info, TokenAccount>,
    /// CHECK: Constrained to the Metaplex Core program ID.
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct StakingPool {
    pub admin: Pubkey,
    pub collection: Pubkey,
    pub reward_mint: Pubkey,
    pub rewards_per_second: u64,
    pub burn_bonus: u64,
    pub total_staked: u64,
    pub authority_bump: u8,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct StakeRecord {
    pub owner: Pubkey,
    pub asset: Pubkey,
    pub collection: Pubkey,
    pub staked_at: i64,
    pub last_claimed_at: i64,
    pub bump: u8,
}

fn accrued_rewards(pool: &StakingPool, stake: &StakeRecord, now: i64) -> Result<u64> {
    let elapsed = now
        .checked_sub(stake.last_claimed_at)
        .ok_or(VaultError::RewardCalculationOverflow)?;
    let elapsed = u64::try_from(elapsed).map_err(|_| error!(VaultError::InvalidClock))?;
    elapsed
        .checked_mul(pool.rewards_per_second)
        .ok_or_else(|| error!(VaultError::RewardCalculationOverflow))
}

fn update_collection_count<'info>(
    core_program: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    total_staked: u64,
    authority_seeds: &[&[u8]],
) -> Result<()> {
    let mut data = vec![7];
    data.extend(encode_attributes_plugin(total_staked));
    invoke_core(
        core_program,
        vec![
            AccountMeta::new(collection.key(), false),
            AccountMeta::new(payer.key(), true),
            AccountMeta::new_readonly(authority.key(), true),
            AccountMeta::new_readonly(system_program.key(), false),
            AccountMeta::new_readonly(CORE_PROGRAM_ID, false),
        ],
        vec![
            collection.clone(),
            payer.clone(),
            authority.clone(),
            system_program.clone(),
            core_program.clone(),
        ],
        data,
        &[authority_seeds],
    )
}

fn encode_attributes_plugin(total_staked: u64) -> Vec<u8> {
    let mut data = vec![6];
    data.extend_from_slice(&1u32.to_le_bytes());
    append_borsh_string(&mut data, "total_staked");
    append_borsh_string(&mut data, &total_staked.to_string());
    data
}

fn append_borsh_string(data: &mut Vec<u8>, value: &str) {
    data.extend_from_slice(&(value.len() as u32).to_le_bytes());
    data.extend_from_slice(value.as_bytes());
}

fn append_init_authority(data: &mut Vec<u8>, authority: Pubkey) {
    data.push(1);
    data.push(3);
    data.extend_from_slice(authority.as_ref());
}

fn invoke_core_add_collection_plugin<'info>(
    core_program: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    plugin_data: Vec<u8>,
    init_authority: Pubkey,
) -> Result<()> {
    let mut data = vec![3];
    data.extend(plugin_data);
    append_init_authority(&mut data, init_authority);
    invoke_core(
        core_program,
        vec![
            AccountMeta::new(collection.key(), false),
            AccountMeta::new(payer.key(), true),
            AccountMeta::new_readonly(authority.key(), true),
            AccountMeta::new_readonly(system_program.key(), false),
            AccountMeta::new_readonly(CORE_PROGRAM_ID, false),
        ],
        vec![
            collection.clone(),
            payer.clone(),
            authority.clone(),
            system_program.clone(),
            core_program.clone(),
        ],
        data,
        &[],
    )
}

fn invoke_core_add_asset_plugin<'info>(
    core_program: &AccountInfo<'info>,
    asset: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    plugin_data: Vec<u8>,
    init_authority: Pubkey,
) -> Result<()> {
    let mut data = vec![2];
    data.extend(plugin_data);
    append_init_authority(&mut data, init_authority);
    invoke_core(
        core_program,
        vec![
            AccountMeta::new(asset.key(), false),
            AccountMeta::new(collection.key(), false),
            AccountMeta::new(payer.key(), true),
            AccountMeta::new_readonly(authority.key(), true),
            AccountMeta::new_readonly(system_program.key(), false),
            AccountMeta::new_readonly(CORE_PROGRAM_ID, false),
        ],
        vec![
            asset.clone(),
            collection.clone(),
            payer.clone(),
            authority.clone(),
            system_program.clone(),
            core_program.clone(),
        ],
        data,
        &[],
    )
}

fn invoke_core_update_asset_plugin<'info>(
    core_program: &AccountInfo<'info>,
    asset: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    plugin_data: Vec<u8>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut data = vec![6];
    data.extend(plugin_data);
    invoke_core(
        core_program,
        vec![
            AccountMeta::new(asset.key(), false),
            AccountMeta::new(collection.key(), false),
            AccountMeta::new(payer.key(), true),
            AccountMeta::new_readonly(authority.key(), true),
            AccountMeta::new_readonly(system_program.key(), false),
            AccountMeta::new_readonly(CORE_PROGRAM_ID, false),
        ],
        vec![
            asset.clone(),
            collection.clone(),
            payer.clone(),
            authority.clone(),
            system_program.clone(),
            core_program.clone(),
        ],
        data,
        signer_seeds,
    )
}

fn invoke_core_burn_asset<'info>(
    core_program: &AccountInfo<'info>,
    asset: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    invoke_core(
        core_program,
        vec![
            AccountMeta::new(asset.key(), false),
            AccountMeta::new(collection.key(), false),
            AccountMeta::new(payer.key(), true),
            AccountMeta::new_readonly(authority.key(), true),
            AccountMeta::new_readonly(system_program.key(), false),
            AccountMeta::new_readonly(CORE_PROGRAM_ID, false),
        ],
        vec![
            asset.clone(),
            collection.clone(),
            payer.clone(),
            authority.clone(),
            system_program.clone(),
            core_program.clone(),
        ],
        vec![12, 0],
        signer_seeds,
    )
}

fn invoke_core_remove_asset_plugin<'info>(
    core_program: &AccountInfo<'info>,
    asset: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    plugin_type: u8,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    invoke_core(
        core_program,
        vec![
            AccountMeta::new(asset.key(), false),
            AccountMeta::new(collection.key(), false),
            AccountMeta::new(payer.key(), true),
            AccountMeta::new_readonly(authority.key(), true),
            AccountMeta::new_readonly(system_program.key(), false),
            AccountMeta::new_readonly(CORE_PROGRAM_ID, false),
        ],
        vec![
            asset.clone(),
            collection.clone(),
            payer.clone(),
            authority.clone(),
            system_program.clone(),
            core_program.clone(),
        ],
        vec![4, plugin_type],
        signer_seeds,
    )
}

fn invoke_core<'info>(
    core_program: &AccountInfo<'info>,
    accounts: Vec<AccountMeta>,
    mut account_infos: Vec<AccountInfo<'info>>,
    data: Vec<u8>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    require_keys_eq!(
        core_program.key(),
        CORE_PROGRAM_ID,
        VaultError::InvalidCoreProgram
    );
    account_infos.push(core_program.clone());
    invoke_signed(
        &Instruction {
            program_id: CORE_PROGRAM_ID,
            accounts,
            data,
        },
        &account_infos,
        signer_seeds,
    )?;
    Ok(())
}

#[error_code]
pub enum VaultError {
    #[msg("Vault already exists")]
    VaultAlreadyExists,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("No staked assets remain in the pool")]
    NoStakedAssets,
    #[msg("No rewards are available to claim")]
    NoRewardsAvailable,
    #[msg("The signer does not own this stake")]
    Unauthorized,
    #[msg("The stake record does not match the provided asset")]
    InvalidStakeRecord,
    #[msg("The on-chain clock is invalid")]
    InvalidClock,
    #[msg("Reward calculation overflow")]
    RewardCalculationOverflow,
    #[msg("Invalid Metaplex Core program")]
    InvalidCoreProgram,
}
