use anchor_lang::prelude::*;
use crate::state::{BettingPool, RoundAccounting, MatchOutcome};
use crate::errors::SportsbookError;
use crate::constants::*;

#[derive(Accounts)]
#[instruction(round_id: u64)]
pub struct SettleRound<'info> {
    #[account(mut)]
    pub betting_pool: Account<'info, BettingPool>,

    #[account(
        mut,
        seeds = [b"round", betting_pool.key().as_ref(), round_id.to_le_bytes().as_ref()],
        bump = round_accounting.bump,
        constraint = round_accounting.seeded @ SportsbookError::RoundNotSeeded,
        constraint = !round_accounting.settled @ SportsbookError::RoundAlreadySettled,
    )]
    pub round_accounting: Account<'info, RoundAccounting>,

    #[account(mut, constraint = authority.key() == betting_pool.authority)]
    pub authority: Signer<'info>,
}

pub fn handler(
    ctx: Context<SettleRound>,
    round_id: u64,
    match_results: Vec<u8>,
) -> Result<()> {
    require!(
        match_results.len() == MATCHES_PER_ROUND,
        SportsbookError::InvalidBetCount
    );

    let round_accounting = &mut ctx.accounts.round_accounting;

    // Validate and store match results
    for i in 0..MATCHES_PER_ROUND {
        let result = match_results[i];
        require!(
            result >= 1 && result <= 3,
            SportsbookError::InvalidOutcome
        );

        round_accounting.match_results[i] = match result {
            1 => MatchOutcome::HomeWin,
            2 => MatchOutcome::AwayWin,
            3 => MatchOutcome::Draw,
            _ => MatchOutcome::Pending,
        };
    }

    // Calculate winning and losing pools
    for match_index in 0..MATCHES_PER_ROUND {
        let match_result = &round_accounting.match_results[match_index];
        let pool = &round_accounting.match_pools[match_index];

        let (winning_pool, losing_pool) = match match_result {
            MatchOutcome::HomeWin => (
                pool.home_win_pool,
                pool.away_win_pool + pool.draw_pool,
            ),
            MatchOutcome::AwayWin => (
                pool.away_win_pool,
                pool.home_win_pool + pool.draw_pool,
            ),
            MatchOutcome::Draw => (
                pool.draw_pool,
                pool.home_win_pool + pool.away_win_pool,
            ),
            MatchOutcome::Pending => (0, 0),
        };

        round_accounting.total_winning_pool += winning_pool;
        round_accounting.total_losing_pool += losing_pool;
    }

    // Calculate total owed to winners using locked odds
    let mut total_owed = 0u64;
    for match_index in 0..MATCHES_PER_ROUND {
        let match_result = &round_accounting.match_results[match_index];
        let pool = &round_accounting.match_pools[match_index];
        let locked_odds = &round_accounting.locked_odds[match_index];

        if *match_result == MatchOutcome::Pending {
            continue;
        }

        let outcome_u8 = match match_result {
            MatchOutcome::HomeWin => 1,
            MatchOutcome::AwayWin => 2,
            MatchOutcome::Draw => 3,
            _ => 0,
        };

        let winning_pool = pool.get_pool_amount(outcome_u8);
        if winning_pool == 0 {
            continue;
        }

        let odds = locked_odds.get_odds(outcome_u8);

        // Total owed = winning pool × locked odds
        let owed_for_match = (winning_pool as u128)
            .checked_mul(odds as u128)
            .ok_or(SportsbookError::CalculationOverflow)?
            .checked_div(ODDS_SCALE as u128)
            .ok_or(SportsbookError::CalculationOverflow)? as u64;

        total_owed += owed_for_match;
    }

    round_accounting.total_reserved_for_winners = total_owed;
    round_accounting.settled = true;
    round_accounting.round_end_time = Clock::get()?.unix_timestamp;

    msg!("Round {} settled", round_id);
    msg!("Total winning pool: {}", round_accounting.total_winning_pool);
    msg!("Total losing pool: {}", round_accounting.total_losing_pool);
    msg!("Total reserved for winners: {}", total_owed);

    Ok(())
}
