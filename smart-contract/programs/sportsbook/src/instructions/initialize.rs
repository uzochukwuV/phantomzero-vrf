use anchor_lang::prelude::*;
use crate::state::{BettingPool, LiquidityPool};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = BettingPool::LEN,
        seeds = [b"betting_pool"],
        bump
    )]
    pub betting_pool: Account<'info, BettingPool>,

    #[account(
        init,
        payer = authority,
        space = LiquidityPool::LEN,
        seeds = [b"liquidity_pool", betting_pool.key().as_ref()],
        bump
    )]
    pub liquidity_pool: Account<'info, LiquidityPool>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// SPL token mint for betting token (e.g., LEAGUE)
    /// CHECK: Token mint is validated by SPL token program
    pub token_mint: UncheckedAccount<'info>,

    /// Protocol treasury for fee collection
    /// CHECK: Treasury address is set by authority
    pub protocol_treasury: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<Initialize>,
    protocol_fee_bps: u16,
    winner_share_bps: u16,
    season_pool_share_bps: u16,
) -> Result<()> {
    // Get keys before mutable borrows
    let betting_pool_key = ctx.accounts.betting_pool.key();
    let liquidity_pool_key = ctx.accounts.liquidity_pool.key();
    let betting_pool_bump = ctx.bumps.betting_pool;
    let liquidity_pool_bump = ctx.bumps.liquidity_pool;

    // Initialize betting pool
    ctx.accounts.betting_pool.authority = ctx.accounts.authority.key();
    ctx.accounts.betting_pool.token_mint = ctx.accounts.token_mint.key();
    ctx.accounts.betting_pool.protocol_treasury = ctx.accounts.protocol_treasury.key();
    ctx.accounts.betting_pool.liquidity_pool = liquidity_pool_key;
    ctx.accounts.betting_pool.protocol_fee_bps = protocol_fee_bps;
    ctx.accounts.betting_pool.winner_share_bps = winner_share_bps;
    ctx.accounts.betting_pool.season_pool_share_bps = season_pool_share_bps;
    ctx.accounts.betting_pool.season_reward_pool = 0;
    ctx.accounts.betting_pool.next_bet_id = 1;
    ctx.accounts.betting_pool.next_round_id = 1;
    ctx.accounts.betting_pool.bump = betting_pool_bump;

    // Initialize liquidity pool
    ctx.accounts.liquidity_pool.betting_pool = betting_pool_key;
    ctx.accounts.liquidity_pool.total_liquidity = 0;
    ctx.accounts.liquidity_pool.total_shares = 0;
    ctx.accounts.liquidity_pool.locked_reserve = 0;
    ctx.accounts.liquidity_pool.available_liquidity = 0;
    ctx.accounts.liquidity_pool.total_profit = 0;
    ctx.accounts.liquidity_pool.total_loss = 0;
    ctx.accounts.liquidity_pool.bump = liquidity_pool_bump;

    msg!("Betting pool initialized successfully");
    msg!("Protocol fee: {}bps", protocol_fee_bps);
    msg!("Winner share: {}bps", winner_share_bps);
    msg!("Season pool share: {}bps", season_pool_share_bps);

    Ok(())
}
