use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{BettingPool, RoundAccounting, LiquidityPool};
use crate::errors::SportsbookError;
use crate::constants::*;
use crate::utils::{calculate_pseudo_random_seeds, calculate_locked_odds_from_seeds};

#[derive(Accounts)]
#[instruction(round_id: u64)]
pub struct SeedRoundPools<'info> {
    #[account(mut)]
    pub betting_pool: Account<'info, BettingPool>,

    #[account(
        mut,
        seeds = [b"round", betting_pool.key().as_ref(), round_id.to_le_bytes().as_ref()],
        bump = round_accounting.bump,
        constraint = !round_accounting.seeded @ SportsbookError::RoundAlreadySeeded,
        constraint = !round_accounting.settled @ SportsbookError::RoundAlreadySettled,
    )]
    pub round_accounting: Account<'info, RoundAccounting>,

    #[account(
        mut,
        seeds = [b"liquidity_pool", betting_pool.key().as_ref()],
        bump = liquidity_pool.bump,
    )]
    pub liquidity_pool: Account<'info, LiquidityPool>,

    /// LP pool's token account
    #[account(mut)]
    pub lp_token_account: Account<'info, TokenAccount>,

    /// Betting pool's token account (receives seed funds)
    #[account(mut)]
    pub betting_pool_token_account: Account<'info, TokenAccount>,

    #[account(mut, constraint = authority.key() == betting_pool.authority)]
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<SeedRoundPools>, round_id: u64) -> Result<()> {
    let mut total_seed_amount = 0u64;

    // Seed each match with DIFFERENTIATED amounts based on team matchup
    // For now, using pseudo-random seeding (would integrate with game engine for team IDs)
    for match_index in 0..MATCHES_PER_ROUND {
        // In production, these would come from a game engine
        // For now, using match_index as a pseudo team ID
        let home_team_id = (match_index as u64) * 2;
        let away_team_id = (match_index as u64) * 2 + 1;

        let (home_seed, away_seed, draw_seed) = calculate_pseudo_random_seeds(
            home_team_id,
            away_team_id,
            round_id,
        );

        // Update match pool with seeds
        let pool = &mut ctx.accounts.round_accounting.match_pools[match_index];
        pool.home_win_pool = home_seed;
        pool.away_win_pool = away_seed;
        pool.draw_pool = draw_seed;
        pool.total_pool = home_seed + away_seed + draw_seed;

        total_seed_amount += pool.total_pool;
        ctx.accounts.round_accounting.total_bet_volume += pool.total_pool;

        // Lock odds based on seed ratios
        let (home_odds, away_odds, draw_odds) = calculate_locked_odds_from_seeds(
            home_seed,
            away_seed,
            draw_seed,
        );

        let locked_odds = &mut ctx.accounts.round_accounting.locked_odds[match_index];
        locked_odds.home_odds = home_odds;
        locked_odds.away_odds = away_odds;
        locked_odds.draw_odds = draw_odds;
        locked_odds.locked = true;

        msg!(
            "Match {}: Seeded with {}/{}/{} tokens, Locked odds: {}/{}/{}",
            match_index,
            home_seed,
            away_seed,
            draw_seed,
            home_odds,
            away_odds,
            draw_odds
        );
    }

    // Check if LP pool can fund seeding
    require!(
        ctx.accounts.liquidity_pool.can_cover_payout(total_seed_amount),
        SportsbookError::InsufficientLPLiquidity
    );

    // Get keys and bump before CPI
    let betting_pool_key = ctx.accounts.betting_pool.key();
    let lp_bump = ctx.accounts.liquidity_pool.bump;
    let lp_account_info = ctx.accounts.liquidity_pool.to_account_info();

    // Transfer seed funds from LP pool to betting pool
    // This uses a Cross-Program Invocation (CPI) with PDA signer
    let seeds = &[b"liquidity_pool", betting_pool_key.as_ref(), &[lp_bump]];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.lp_token_account.to_account_info(),
        to: ctx.accounts.betting_pool_token_account.to_account_info(),
        authority: lp_account_info,
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

    token::transfer(cpi_ctx, total_seed_amount)?;

    // Update LP pool state
    ctx.accounts.liquidity_pool.total_liquidity -= total_seed_amount;
    ctx.accounts.liquidity_pool.available_liquidity = ctx.accounts.liquidity_pool
        .total_liquidity
        .saturating_sub(ctx.accounts.liquidity_pool.locked_reserve);

    // Update round accounting
    ctx.accounts.round_accounting.protocol_seed_amount = total_seed_amount;
    ctx.accounts.round_accounting.seeded = true;

    msg!("Round {} seeded with {} tokens total", round_id, total_seed_amount);
    msg!("Odds locked for all matches");

    Ok(())
}
