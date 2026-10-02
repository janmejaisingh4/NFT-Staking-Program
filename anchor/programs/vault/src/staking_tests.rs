use crate::{
    accrued_rewards, append_init_authority, encode_attributes_plugin, StakeRecord, StakingPool,
    CORE_PROGRAM_ID, ID as PROGRAM_ID,
};
use anchor_lang::{
    prelude::Pubkey, solana_program::program_pack::Pack, AccountDeserialize, InstructionData,
    ToAccountMetas,
};
use anchor_spl::token::spl_token::state::Account as SplTokenAccount;
use litesvm::LiteSVM;
use solana_program_runtime::{
    declare_process_instruction, invoke_context::BuiltinFunctionWithContext,
};
use solana_sdk::{
    account::Account, clock::Clock, instruction::Instruction, native_loader, signature::Keypair,
    signer::Signer, transaction::Transaction,
};

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
const REWARDS_PER_SECOND: u64 = 2;
const BURN_BONUS: u64 = 1_000;

// The test double accepts Core CPIs so LiteSVM can exercise the vault's
// instruction flow without downloading or vendoring the Core program binary.
declare_process_instruction!(MockCoreProgram, 1, |_invoke_context| { Ok(()) });

fn build_instruction<A: ToAccountMetas, D: InstructionData>(accounts: A, data: D) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

fn send_instruction(
    svm: &mut LiteSVM,
    payer: &Keypair,
    signers: &[&dyn Signer],
    name: &str,
    instruction: Instruction,
) -> Vec<String> {
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        signers,
        svm.latest_blockhash(),
    );
    let metadata = svm.send_transaction(transaction).unwrap_or_else(|failure| {
        panic!(
            "{name} failed: {:?}\n{}",
            failure.err,
            failure.meta.pretty_logs()
        )
    });
    let pretty_logs = metadata.pretty_logs();
    println!("\n=== {name} ===\n{pretty_logs}");
    metadata.logs
}

fn assert_instruction_log(logs: &[String], instruction_name: &str) {
    let expected = format!("Instruction: {instruction_name}");
    assert!(
        logs.iter().any(|log| log.contains(&expected)),
        "missing `{expected}` in logs:\n{}",
        logs.join("\n")
    );
}

fn assert_core_cpi_count(logs: &[String], expected: usize) {
    let core_invocations = logs
        .iter()
        .filter(|log| log.contains(&format!("Program {CORE_PROGRAM_ID} invoke")))
        .count();
    assert_eq!(core_invocations, expected, "{}", logs.join("\n"));
}

fn set_clock(svm: &mut LiteSVM, unix_timestamp: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = unix_timestamp;
    svm.set_sysvar::<Clock>(&clock);
}

fn set_core_owned_account(svm: &mut LiteSVM, address: Pubkey) {
    svm.set_account(
        address,
        Account {
            lamports: svm.minimum_balance_for_rent_exemption(0),
            data: vec![],
            owner: CORE_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}

fn pool_state(svm: &LiteSVM, address: Pubkey) -> StakingPool {
    let account = svm
        .get_account(&address)
        .expect("pool account should exist");
    let mut data = account.data.as_slice();
    StakingPool::try_deserialize(&mut data).expect("pool account should deserialize")
}

fn derive_pool(collection: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"pool", collection.as_ref()], &PROGRAM_ID)
}

fn derive_authority(collection: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"pool-authority", collection.as_ref()], &PROGRAM_ID)
}

fn derive_stake(asset: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"stake", asset.as_ref()], &PROGRAM_ID)
}

fn initialize_pool_instruction(
    admin: Pubkey,
    collection: Pubkey,
    pool: Pubkey,
    authority: Pubkey,
    reward_mint: Pubkey,
) -> Instruction {
    build_instruction(
        crate::accounts::InitializePool {
            admin,
            collection,
            pool,
            authority,
            reward_mint,
            core_program: CORE_PROGRAM_ID,
            token_program: anchor_spl::token::ID,
            system_program: anchor_lang::system_program::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
        },
        crate::instruction::InitializePool {
            rewards_per_second: REWARDS_PER_SECOND,
            burn_bonus: BURN_BONUS,
        },
    )
}

fn stake_instruction(user: Pubkey, pool: Pubkey, collection: Pubkey, asset: Pubkey) -> Instruction {
    let (stake_record, _) = derive_stake(asset);
    let (authority, _) = derive_authority(collection);
    build_instruction(
        crate::accounts::StakeNft {
            user,
            pool,
            collection,
            asset,
            stake_record,
            authority,
            core_program: CORE_PROGRAM_ID,
            system_program: anchor_lang::system_program::ID,
        },
        crate::instruction::StakeNft {},
    )
}

fn claim_instruction(
    user: Pubkey,
    pool: Pubkey,
    collection: Pubkey,
    asset: Pubkey,
    reward_mint: Pubkey,
) -> Instruction {
    let (stake_record, _) = derive_stake(asset);
    let (authority, _) = derive_authority(collection);
    build_instruction(
        crate::accounts::ClaimRewards {
            user,
            pool,
            collection,
            reward_mint,
            stake_record,
            asset,
            authority,
            user_reward_account: anchor_spl::associated_token::get_associated_token_address(
                &user,
                &reward_mint,
            ),
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::system_program::ID,
        },
        crate::instruction::ClaimRewards {},
    )
}

fn unstake_instruction(
    user: Pubkey,
    pool: Pubkey,
    collection: Pubkey,
    asset: Pubkey,
    reward_mint: Pubkey,
) -> Instruction {
    let (stake_record, _) = derive_stake(asset);
    let (authority, _) = derive_authority(collection);
    build_instruction(
        crate::accounts::UnstakeNft {
            user,
            pool,
            collection,
            asset,
            stake_record,
            reward_mint,
            authority,
            user_reward_account: anchor_spl::associated_token::get_associated_token_address(
                &user,
                &reward_mint,
            ),
            core_program: CORE_PROGRAM_ID,
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::system_program::ID,
        },
        crate::instruction::UnstakeNft {},
    )
}

fn burn_instruction(
    user: Pubkey,
    pool: Pubkey,
    collection: Pubkey,
    asset: Pubkey,
    reward_mint: Pubkey,
) -> Instruction {
    let (stake_record, _) = derive_stake(asset);
    let (authority, _) = derive_authority(collection);
    build_instruction(
        crate::accounts::BurnStakedNft {
            user,
            pool,
            collection,
            asset,
            stake_record,
            reward_mint,
            authority,
            user_reward_account: anchor_spl::associated_token::get_associated_token_address(
                &user,
                &reward_mint,
            ),
            core_program: CORE_PROGRAM_ID,
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::system_program::ID,
        },
        crate::instruction::BurnStakedNft {},
    )
}

#[test]
fn staking_and_unstaking_lifecycle_logs_each_instruction() {
    let mut svm = LiteSVM::new().with_default_programs();
    svm.add_program(
        PROGRAM_ID,
        include_bytes!("../../../target/deploy/vault.so"),
    )
    .unwrap();
    let core_processor: BuiltinFunctionWithContext = MockCoreProgram::vm;
    svm.add_builtin(CORE_PROGRAM_ID, core_processor);
    svm.set_account(
        CORE_PROGRAM_ID,
        Account {
            lamports: 1,
            data: vec![],
            owner: native_loader::ID,
            executable: true,
            rent_epoch: 0,
        },
    )
    .unwrap();

    let user = Keypair::new();
    let reward_mint = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
    set_clock(&mut svm, 1_700_000_000);

    let collection = Pubkey::new_unique();
    let first_asset = Pubkey::new_unique();
    let second_asset = Pubkey::new_unique();
    set_core_owned_account(&mut svm, collection);
    set_core_owned_account(&mut svm, first_asset);
    set_core_owned_account(&mut svm, second_asset);

    let (pool, _) = derive_pool(collection);
    let (authority, _) = derive_authority(collection);

    let initialize_logs = send_instruction(
        &mut svm,
        &user,
        &[&user, &reward_mint],
        "initialize_pool",
        initialize_pool_instruction(
            user.pubkey(),
            collection,
            pool,
            authority,
            reward_mint.pubkey(),
        ),
    );
    assert_instruction_log(&initialize_logs, "InitializePool");
    assert_core_cpi_count(&initialize_logs, 1);

    let stake_logs = send_instruction(
        &mut svm,
        &user,
        &[&user],
        "stake_nft",
        stake_instruction(user.pubkey(), pool, collection, first_asset),
    );
    assert_instruction_log(&stake_logs, "StakeNft");
    assert_core_cpi_count(&stake_logs, 3);
    assert_eq!(pool_state(&svm, pool).total_staked, 1);

    set_clock(&mut svm, 1_700_000_010);
    let claim_logs = send_instruction(
        &mut svm,
        &user,
        &[&user],
        "claim_rewards",
        claim_instruction(
            user.pubkey(),
            pool,
            collection,
            first_asset,
            reward_mint.pubkey(),
        ),
    );
    assert_instruction_log(&claim_logs, "ClaimRewards");
    assert_core_cpi_count(&claim_logs, 0);

    set_clock(&mut svm, 1_700_000_015);
    let unstake_logs = send_instruction(
        &mut svm,
        &user,
        &[&user],
        "unstake_nft",
        unstake_instruction(
            user.pubkey(),
            pool,
            collection,
            first_asset,
            reward_mint.pubkey(),
        ),
    );
    assert_instruction_log(&unstake_logs, "UnstakeNft");
    assert_core_cpi_count(&unstake_logs, 4);
    assert_eq!(pool_state(&svm, pool).total_staked, 0);
    let (first_stake, _) = derive_stake(first_asset);
    assert!(svm.get_account(&first_stake).is_none());

    set_clock(&mut svm, 1_700_000_020);
    let second_stake_logs = send_instruction(
        &mut svm,
        &user,
        &[&user],
        "stake_second_nft",
        stake_instruction(user.pubkey(), pool, collection, second_asset),
    );
    assert_instruction_log(&second_stake_logs, "StakeNft");
    assert_core_cpi_count(&second_stake_logs, 3);

    set_clock(&mut svm, 1_700_000_025);
    let burn_logs = send_instruction(
        &mut svm,
        &user,
        &[&user],
        "burn_staked_nft",
        burn_instruction(
            user.pubkey(),
            pool,
            collection,
            second_asset,
            reward_mint.pubkey(),
        ),
    );
    assert_instruction_log(&burn_logs, "BurnStakedNft");
    assert_core_cpi_count(&burn_logs, 2);
    assert_eq!(pool_state(&svm, pool).total_staked, 0);

    let reward_account = anchor_spl::associated_token::get_associated_token_address(
        &user.pubkey(),
        &reward_mint.pubkey(),
    );
    let reward_account = svm
        .get_account(&reward_account)
        .expect("reward ATA should exist");
    let token_account = SplTokenAccount::unpack(&reward_account.data).unwrap();
    assert_eq!(token_account.amount, 1_040);
}

fn sample_pool(rewards_per_second: u64) -> StakingPool {
    StakingPool {
        admin: Pubkey::default(),
        collection: Pubkey::default(),
        reward_mint: Pubkey::default(),
        rewards_per_second,
        burn_bonus: 1,
        total_staked: 1,
        authority_bump: 0,
        bump: 0,
    }
}

fn sample_stake(last_claimed_at: i64) -> StakeRecord {
    StakeRecord {
        owner: Pubkey::default(),
        asset: Pubkey::default(),
        collection: Pubkey::default(),
        staked_at: last_claimed_at,
        last_claimed_at,
        bump: 0,
    }
}

#[test]
fn reward_accrual_uses_elapsed_seconds_and_rate() {
    let rewards = accrued_rewards(&sample_pool(7), &sample_stake(100), 130).unwrap();
    assert_eq!(rewards, 210);
}

#[test]
fn reward_accrual_rejects_a_backwards_clock() {
    assert!(accrued_rewards(&sample_pool(7), &sample_stake(100), 99).is_err());
}

#[test]
fn reward_accrual_rejects_overflow() {
    assert!(accrued_rewards(&sample_pool(u64::MAX), &sample_stake(0), i64::MAX).is_err());
}

#[test]
fn core_attributes_plugin_uses_core_borsh_layout() {
    let mut expected = vec![6, 1, 0, 0, 0, 12, 0, 0, 0];
    expected.extend_from_slice(b"total_staked");
    expected.extend_from_slice(&[1, 0, 0, 0, b'0']);
    assert_eq!(encode_attributes_plugin(0), expected);
}

#[test]
fn core_plugin_authority_encodes_some_address_variant() {
    let address = Pubkey::new_from_array([7; 32]);
    let mut encoded = Vec::new();
    append_init_authority(&mut encoded, address);
    assert_eq!(encoded, [&[1, 3][..], address.as_ref()].concat());
}
