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

    let liquidity_pool = &mut ctx.accounts.liquidity_pool;
    let lp_position = &mut ctx.accounts.lp_position;

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
    let shares = liquidity_pool.add_liquidity(amount);

    // Initialize or update LP position
    if lp_position.owner == Pubkey::default() {
        lp_position.owner = ctx.accounts.provider.key();
        lp_position.liquidity_pool = ctx.accounts.liquidity_pool.key();
        lp_position.shares = shares;
        lp_position.bump = ctx.bumps.lp_position;
    } else {
        lp_position.shares += shares;
    }

    msg!("Added {} tokens to LP, received {} shares", amount, shares);
    msg!("Total LP liquidity: {}", liquidity_pool.total_liquidity);
    msg!("Total LP shares: {}", liquidity_pool.total_shares);

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

    let liquidity_pool = &mut ctx.accounts.liquidity_pool;
    let lp_position = &mut ctx.accounts.lp_position;

    require!(
        lp_position.shares >= shares,
        SportsbookError::InvalidAmount
    );

    // Calculate withdrawal amount
    let amount = liquidity_pool.calculate_withdrawal(shares);

    require!(
        liquidity_pool.available_liquidity >= amount,
        SportsbookError::InsufficientAvailableLiquidity
    );

    // Update LP pool state
    liquidity_pool.remove_liquidity(shares);

    // Update LP position
    lp_position.shares -= shares;

    // Transfer tokens from LP pool to provider
    let betting_pool_key = ctx.accounts.betting_pool.key();
    let seeds = &[b"liquidity_pool", betting_pool_key.as_ref(), &[liquidity_pool.bump]];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.lp_token_account.to_account_info(),
        to: ctx.accounts.provider_token_account.to_account_info(),
        authority: ctx.accounts.liquidity_pool.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    token::transfer(cpi_ctx, amount)?;

    msg!("Removed {} shares from LP, received {} tokens", shares, amount);
    msg!("Total LP liquidity: {}", liquidity_pool.total_liquidity);
    msg!("Remaining shares: {}", lp_position.shares);

    Ok(())
}
