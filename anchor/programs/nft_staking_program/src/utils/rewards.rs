use anchor_lang::prelude::*;
use anchor_spl::token::{self, MintTo};

use crate::{StakeRecord, StakingError};

pub fn pending_rewards(rec: &StakeRecord, per_sec: u64, now: i64) -> Result<u64> {
    let elapsed = now.saturating_sub(rec.last_claim).max(0) as u64;
    elapsed
        .checked_mul(per_sec)
        .ok_or_else(|| error!(StakingError::MathOverflow))
}

pub fn mint_rewards<'a>(
    token_program: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    seeds: &[&[u8]],
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }

    token::mint_to(
        CpiContext::new_with_signer(
            token_program.clone(),
            MintTo {
                mint: mint.clone(),
                to: to.clone(),
                authority: authority.clone(),
            },
            &[seeds],
        ),
        amount,
    )
}
