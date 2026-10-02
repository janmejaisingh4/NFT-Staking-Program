#[cfg(test)]
mod tests {
    use crate::{
        accrued_rewards, append_init_authority, encode_attributes_plugin, StakeRecord, StakingPool,
        ID as PROGRAM_ID,
    };
    use litesvm::LiteSVM;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        system_program,
        transaction::Transaction,
    };

    const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

    fn get_vault_pda(signer: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault", signer.as_ref()], &PROGRAM_ID)
    }

    fn create_deposit_ix(signer: &Pubkey, vault: &Pubkey, amount: u64) -> Instruction {
        // Anchor discriminator for "deposit" = hash("global:deposit")[0..8]
        let discriminator: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
        let mut data = discriminator.to_vec();
        data.extend_from_slice(&amount.to_le_bytes());

        Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(*signer, true),
                AccountMeta::new(*vault, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data,
        }
    }

    fn create_withdraw_ix(signer: &Pubkey, vault: &Pubkey) -> Instruction {
        // Anchor discriminator for "withdraw" = hash("global:withdraw")[0..8]
        let discriminator: [u8; 8] = [183, 18, 70, 156, 148, 109, 161, 34];

        Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(*signer, true),
                AccountMeta::new(*vault, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: discriminator.to_vec(),
        }
    }

    #[test]
    fn test_deposit_and_withdraw() {
        let mut svm = LiteSVM::new();

        // Load the program
        let program_bytes = include_bytes!("../../../target/deploy/vault.so");
        svm.add_program(PROGRAM_ID, program_bytes);

        // Create a user with some SOL
        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        // Get vault PDA
        let (vault_pda, _bump) = get_vault_pda(&user.pubkey());

        // Deposit 1 SOL
        let deposit_amount = LAMPORTS_PER_SOL;
        let deposit_ix = create_deposit_ix(&user.pubkey(), &vault_pda, deposit_amount);

        let blockhash = svm.latest_blockhash();
        let deposit_tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&user.pubkey()),
            &[&user],
            blockhash,
        );

        let result = svm.send_transaction(deposit_tx);
        assert!(result.is_ok(), "Deposit should succeed");

        // Check vault balance
        let vault_account = svm.get_account(&vault_pda).unwrap();
        assert_eq!(vault_account.lamports, deposit_amount);

        // Withdraw
        let withdraw_ix = create_withdraw_ix(&user.pubkey(), &vault_pda);

        let blockhash = svm.latest_blockhash();
        let withdraw_tx = Transaction::new_signed_with_payer(
            &[withdraw_ix],
            Some(&user.pubkey()),
            &[&user],
            blockhash,
        );

        let result = svm.send_transaction(withdraw_tx);
        assert!(result.is_ok(), "Withdraw should succeed");

        // Check vault is empty (account may not exist or have 0 lamports)
        let vault_account = svm.get_account(&vault_pda);
        assert!(
            vault_account.is_none() || vault_account.unwrap().lamports == 0,
            "Vault should be empty after withdraw"
        );
    }

    #[test]
    fn test_deposit_fails_if_vault_has_funds() {
        let mut svm = LiteSVM::new();

        let program_bytes = include_bytes!("../../../target/deploy/vault.so");
        svm.add_program(PROGRAM_ID, program_bytes);

        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let (vault_pda, _bump) = get_vault_pda(&user.pubkey());

        // First deposit
        let deposit_ix = create_deposit_ix(&user.pubkey(), &vault_pda, LAMPORTS_PER_SOL);
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&user.pubkey()),
            &[&user],
            blockhash,
        );
        svm.send_transaction(tx).unwrap();

        // Second deposit should fail
        let deposit_ix2 = create_deposit_ix(&user.pubkey(), &vault_pda, LAMPORTS_PER_SOL);
        let blockhash = svm.latest_blockhash();
        let tx2 = Transaction::new_signed_with_payer(
            &[deposit_ix2],
            Some(&user.pubkey()),
            &[&user],
            blockhash,
        );

        let result = svm.send_transaction(tx2);
        assert!(result.is_err(), "Second deposit should fail");
    }

    #[test]
    fn test_withdraw_fails_if_vault_empty() {
        let mut svm = LiteSVM::new();

        let program_bytes = include_bytes!("../../../target/deploy/vault.so");
        svm.add_program(PROGRAM_ID, program_bytes);

        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let (vault_pda, _bump) = get_vault_pda(&user.pubkey());

        // Try to withdraw from empty vault
        let withdraw_ix = create_withdraw_ix(&user.pubkey(), &vault_pda);
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[withdraw_ix],
            Some(&user.pubkey()),
            &[&user],
            blockhash,
        );

        let result = svm.send_transaction(tx);
        assert!(result.is_err(), "Withdraw from empty vault should fail");
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
        let result = accrued_rewards(&sample_pool(7), &sample_stake(100), 99);
        assert!(result.is_err());
    }

    #[test]
    fn reward_accrual_rejects_overflow() {
        let result = accrued_rewards(&sample_pool(u64::MAX), &sample_stake(0), i64::MAX);
        assert!(result.is_err());
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
}
