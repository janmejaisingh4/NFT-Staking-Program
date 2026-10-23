use anchor_lang::error::Error;
use anchor_lang::prelude::Pubkey;
use vault::{utils::pending_rewards, StakeRecord, StakingError};

fn stake_record(last_claim: i64) -> StakeRecord {
    StakeRecord {
        owner: Pubkey::new_unique(),
        asset: Pubkey::new_unique(),
        staked_at: last_claim,
        last_claim,
    }
}

#[test]
fn calculates_rewards_for_elapsed_time() {
    let record = stake_record(100);

    assert_eq!(pending_rewards(&record, 3, 160).unwrap(), 180);
}

#[test]
fn returns_zero_when_reward_rate_is_zero() {
    let record = stake_record(100);

    assert_eq!(pending_rewards(&record, 0, 160).unwrap(), 0);
}

#[test]
fn returns_zero_when_no_time_has_elapsed() {
    let record = stake_record(100);

    assert_eq!(pending_rewards(&record, 5, 100).unwrap(), 0);
}

#[test]
fn returns_zero_when_current_time_precedes_last_claim() {
    let record = stake_record(100);

    assert_eq!(pending_rewards(&record, 5, 90).unwrap(), 0);
}

#[test]
fn saturates_elapsed_time_when_timestamps_span_i64_range() {
    let record = stake_record(i64::MIN);

    assert_eq!(
        pending_rewards(&record, 1, i64::MAX).unwrap(),
        i64::MAX as u64
    );
}

#[test]
fn reports_math_overflow_when_reward_multiplication_overflows() {
    let record = stake_record(0);

    let error = pending_rewards(&record, u64::MAX, 2).unwrap_err();

    match error {
        Error::AnchorError(anchor_error) => {
            assert_eq!(anchor_error.error_name, "MathOverflow");
            assert_eq!(
                anchor_error.error_code_number,
                anchor_lang::error::ERROR_CODE_OFFSET + StakingError::MathOverflow as u32
            );
        }
        other => panic!("expected MathOverflow, got {other:?}"),
    }
}
