use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{BettingPool, LiquidityPool, LpPosition};
use crate::errors::SportsbookError;

/// Add liquidity to the LP pool
#[derive(Accounts)]
pub struct AddLiquidity<'info> {
    #[account(mut)]
    pub betting_pool: Account<'info, BettingPool>,

    #[account(
        mut,
        seeds = [b"liquidity_pool", betting_pool.key().as_ref()],
        bump = liquidity_pool.bump,
    )]
    pub liquidity_pool: Account<'info, LiquidityPool>,

    #[account(
        init_if_needed,
        payer = provider,
        space = LpPosition::LEN,
        seeds = [b"lp_position", liquidity_pool.key().as_ref(), provider.key().as_ref()],
        bump
    )]
    pub lp_position: Account<'info, LpPosition>,

    /// Provider's token account
    #[account(mut)]
    pub provider_token_account: Account<'info, TokenAccount>,

    /// LP pool's token account
    #[account(mut)]
    pub lp_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub provider: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn add_liquidity(ctx: Context<AddLiquidity>, amount: u64) -> Result<()> {
    require!(amount > 0, SportsbookError::InvalidAmount);

    // Get keys before mutable borrows
    let liquidity_pool_key = ctx.accounts.liquidity_pool.key();
    let provider_key = ctx.accounts.provider.key();
    let lp_bump = ctx.bumps.lp_position;

    // Transfer tokens from provider to LP pool
    let cpi_accounts = Transfer {
        from: ctx.accounts.provider_token_account.to_account_info(),
        to: ctx.accounts.lp_token_account.to_account_info(),
        authority: ctx.accounts.provider.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    // Calculate shares
    let shares = ctx.accounts.liquidity_pool.add_liquidity(amount);

    // Initialize or update LP position
    if ctx.accounts.lp_position.owner == Pubkey::default() {
        ctx.accounts.lp_position.owner = provider_key;
        ctx.accounts.lp_position.liquidity_pool = liquidity_pool_key;
        ctx.accounts.lp_position.shares = shares;
        ctx.accounts.lp_position.bump = lp_bump;
    } else {
        ctx.accounts.lp_position.shares += shares;
    }

    msg!("Added {} tokens to LP, received {} shares", amount, shares);
    msg!("Total LP liquidity: {}", ctx.accounts.liquidity_pool.total_liquidity);
    msg!("Total LP shares: {}", ctx.accounts.liquidity_pool.total_shares);

    Ok(())
}

/// Remove liquidity from the LP pool
#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    #[account(mut)]
    pub betting_pool: Account<'info, BettingPool>,

    #[account(
        mut,
        seeds = [b"liquidity_pool", betting_pool.key().as_ref()],
        bump = liquidity_pool.bump,
    )]
    pub liquidity_pool: Account<'info, LiquidityPool>,

    #[account(
        mut,
        seeds = [b"lp_position", liquidity_pool.key().as_ref(), provider.key().as_ref()],
        bump = lp_position.bump,
        constraint = lp_position.owner == provider.key() @ SportsbookError::InvalidAuthority,
    )]
    pub lp_position: Account<'info, LpPosition>,

    /// Provider's token account (receives withdrawn tokens)
    #[account(mut)]
    pub provider_token_account: Account<'info, TokenAccount>,

    /// LP pool's token account
    #[account(mut)]
    pub lp_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub provider: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn remove_liquidity(ctx: Context<RemoveLiquidity>, shares: u64) -> Result<()> {
    require!(shares > 0, SportsbookError::InvalidAmount);

    require!(
        ctx.accounts.lp_position.shares >= shares,
        SportsbookError::InvalidAmount
    );

    // Calculate withdrawal amount
    let amount = ctx.accounts.liquidity_pool.calculate_withdrawal(shares);

    require!(
        ctx.accounts.liquidity_pool.available_liquidity >= amount,
        SportsbookError::InsufficientAvailableLiquidity
    );

    // Get keys and bump before mutable borrows
    let betting_pool_key = ctx.accounts.betting_pool.key();
    let lp_bump = ctx.accounts.liquidity_pool.bump;
    let lp_account_info = ctx.accounts.liquidity_pool.to_account_info();

    // Update LP pool state
    ctx.accounts.liquidity_pool.remove_liquidity(shares);

    // Update LP position
    ctx.accounts.lp_position.shares -= shares;

    // Transfer tokens from LP pool to provider
    let seeds = &[b"liquidity_pool", betting_pool_key.as_ref(), &[lp_bump]];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.lp_token_account.to_account_info(),
        to: ctx.accounts.provider_token_account.to_account_info(),
        authority: lp_account_info,
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    token::transfer(cpi_ctx, amount)?;

    msg!("Removed {} shares from LP, received {} tokens", shares, amount);
    msg!("Total LP liquidity: {}", ctx.accounts.liquidity_pool.total_liquidity);
    msg!("Remaining shares: {}", ctx.accounts.lp_position.shares);

    Ok(())
}
