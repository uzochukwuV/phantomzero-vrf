use anchor_lang::prelude::*;

/// Global betting pool configuration and state
#[account]
pub struct BettingPool {
    /// Authority that can manage the pool (owner)
    pub authority: Pubkey,

    /// SPL token mint for the betting token (e.g., LEAGUE token)
    pub token_mint: Pubkey,

    /// Protocol treasury address for fee collection
    pub protocol_treasury: Pubkey,

    /// Liquidity pool PDA
    pub liquidity_pool: Pubkey,

    /// Protocol fee in basis points (e.g., 500 = 5%)
    pub protocol_fee_bps: u16,

    /// Winner share in basis points (e.g., 2500 = 25%)
    pub winner_share_bps: u16,

    /// Season pool share in basis points (e.g., 200 = 2%)
    pub season_pool_share_bps: u16,

    /// Total season reward pool accumulated
    pub season_reward_pool: u64,

    /// Next bet ID counter
    pub next_bet_id: u64,

    /// Next round ID counter
    pub next_round_id: u64,

    /// Bump seed for PDA
    pub bump: u8,
}

impl BettingPool {
    pub const LEN: usize = 8 + // discriminator
        32 + // authority
        32 + // token_mint
        32 + // protocol_treasury
        32 + // liquidity_pool
        2 +  // protocol_fee_bps
        2 +  // winner_share_bps
        2 +  // season_pool_share_bps
        8 +  // season_reward_pool
        8 +  // next_bet_id
        8 +  // next_round_id
        1;   // bump
}
