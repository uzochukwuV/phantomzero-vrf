use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{BettingPool, RoundAccounting};
use crate::errors::SportsbookError;
use crate::constants::*;

#[derive(Accounts)]
#[instruction(round_id: u64)]
pub struct FinalizeRoundRevenue<'info> {
    #[account(mut)]
    pub betting_pool: Account<'info, BettingPool>,

    #[account(
        mut,
        seeds = [b"round", betting_pool.key().as_ref(), round_id.to_le_bytes().as_ref()],
        bump = round_accounting.bump,
        constraint = round_accounting.settled @ SportsbookError::RoundNotSettled,
        constraint = !round_accounting.revenue_distributed @ SportsbookError::RevenueAlreadyDistributed,
    )]
    pub round_accounting: Account<'info, RoundAccounting>,

    /// Betting pool's token account (protocol holds all funds)
    #[account(mut)]
    pub betting_pool_token_account: Account<'info, TokenAccount>,

    #[account(mut, constraint = authority.key() == betting_pool.authority)]
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<FinalizeRoundRevenue>, round_id: u64) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // IMPORTANT: With multi-match parlays, we CANNOT calculate total_reserved_for_winners
    // without iterating through all bets (which defeats the purpose of O(10) accounting).
    //
    // Instead, we use time-based finalization:
    // - Winners have 24 hours to claim (or lose to bounty hunters)
    // - After 24h + buffer (e.g., 1 hour), protocol can finalize revenue
    // - Any unclaimed winnings after this deadline become protocol profit
    //
    // This is acceptable because:
    // 1. Winners have 24h to claim 100%
    // 2. Bounty hunters have incentive to claim for winners (get 10%)
    // 3. After 25 hours, extremely unlikely any unclaimed winners remain

    let claim_deadline = ctx.accounts.round_accounting.round_end_time + 86400; // 24 hours
    let finalize_buffer = 3600; // 1 hour buffer after claim deadline
    let earliest_finalize_time = claim_deadline + finalize_buffer;

    require!(
        current_time >= earliest_finalize_time,
        SportsbookError::RevenueDistributedBeforeClaims
    );

    // Extract season pool share
    let season_pool_share_bps = ctx.accounts.betting_pool.season_pool_share_bps;

    // Check actual balance remaining in betting pool
    let remaining_in_contract = ctx.accounts.betting_pool_token_account.amount;
    let protocol_seed = ctx.accounts.round_accounting.protocol_seed_amount;
    let user_deposits = ctx.accounts.round_accounting.total_user_deposits;
    let total_paid = ctx.accounts.round_accounting.total_paid_out;

    // CORRECT ACCOUNTING:
    // Operating profit/loss = user_deposits - total_paid (can be negative!)
    // Remaining balance = seed + user_deposits - total_paid
    //                   = seed + operating_profit
    //
    // If operating_profit is negative, protocol loses from seed capital

    let mut season_share = 0u64;
    let mut operating_profit = 0i64; // Can be negative!

    if user_deposits > 0 {
        // Season pool gets exactly 2% of ACTUAL USER DEPOSITS (before fee)
        let total_user_bets_before_fee = user_deposits
            .saturating_add(ctx.accounts.round_accounting.protocol_fee_collected);

        season_share = (total_user_bets_before_fee as u128)
            .checked_mul(season_pool_share_bps as u128)
            .ok_or(SportsbookError::CalculationOverflow)?
            .checked_div(BPS_DENOMINATOR as u128)
            .ok_or(SportsbookError::CalculationOverflow)? as u64;

        // Cap season share to what's actually available
        if season_share > remaining_in_contract {
            season_share = remaining_in_contract;
        }

        // Allocate season pool share (stays in betting pool for season rewards)
        if season_share > 0 {
            ctx.accounts.betting_pool.season_reward_pool += season_share;
        }
    }

    // Calculate operating profit (EXCLUDING seed capital)
    // This can be negative if protocol paid out more than users deposited
    operating_profit = user_deposits as i64 - total_paid as i64;

    // Store profit (note: if negative, this represents a loss)
    // For u64 storage, we'll store the absolute value and track sign separately
    // or accept that negative profits show as 0
    let protocol_revenue = if operating_profit > 0 {
        operating_profit as u64
    } else {
        0 // Loss - protocol used seed capital
    };

    ctx.accounts.round_accounting.protocol_revenue_share = protocol_revenue;
    ctx.accounts.round_accounting.season_revenue_share = season_share;
    ctx.accounts.round_accounting.revenue_distributed = true;

    msg!("Round {} revenue finalized", round_id);
    msg!("Protocol seed: {} (stays in pool)", protocol_seed);
    msg!("User deposits: {}", user_deposits);
    msg!("Total paid: {}", total_paid);
    msg!("Operating profit: {} (negative = loss from seed)", operating_profit);
    msg!("Remaining balance: {}", remaining_in_contract);
    msg!("Season share: {}", season_share);

    Ok(())
}
