use anchor_lang::prelude::*;

/// Liquidity Pool for funding bet payouts and round seeding
#[account]
pub struct LiquidityPool {
    /// Betting pool this LP belongs to
    pub betting_pool: Pubkey,

    /// Total liquidity in the pool
    pub total_liquidity: u64,

    /// Total LP shares issued
    pub total_shares: u64,

    /// Locked reserve for pending payouts
    pub locked_reserve: u64,

    /// Available liquidity (total - locked)
    pub available_liquidity: u64,

    /// Total profit earned by LP
    pub total_profit: u64,

    /// Total loss incurred by LP
    pub total_loss: u64,

    /// Bump seed for PDA
    pub bump: u8,
}

impl LiquidityPool {
    pub const LEN: usize = 8 + // discriminator
        32 + // betting_pool
        8 +  // total_liquidity
        8 +  // total_shares
        8 +  // locked_reserve
        8 +  // available_liquidity
        8 +  // total_profit
        8 +  // total_loss
        1;   // bump

    /// Calculate shares for a deposit amount
    pub fn calculate_shares(&self, deposit_amount: u64) -> u64 {
        if self.total_shares == 0 || self.total_liquidity == 0 {
            // First depositor gets 1:1 shares
            deposit_amount
        } else {
            // shares = (deposit_amount * total_shares) / total_liquidity
            (deposit_amount as u128)
                .checked_mul(self.total_shares as u128)
                .unwrap()
                .checked_div(self.total_liquidity as u128)
                .unwrap() as u64
        }
    }

    /// Calculate withdrawal amount for shares
    pub fn calculate_withdrawal(&self, shares: u64) -> u64 {
        if self.total_shares == 0 {
            0
        } else {
            // amount = (shares * total_liquidity) / total_shares
            (shares as u128)
                .checked_mul(self.total_liquidity as u128)
                .unwrap()
                .checked_div(self.total_shares as u128)
                .unwrap() as u64
        }
    }

    /// Check if LP can cover a payout
    pub fn can_cover_payout(&self, amount: u64) -> bool {
        self.available_liquidity >= amount
    }

    /// Lock reserve for a potential payout
    pub fn lock_reserve(&mut self, amount: u64) {
        self.locked_reserve += amount;
        self.available_liquidity = self.total_liquidity.saturating_sub(self.locked_reserve);
    }

    /// Release locked reserve
    pub fn release_reserve(&mut self, amount: u64) {
        self.locked_reserve = self.locked_reserve.saturating_sub(amount);
        self.available_liquidity = self.total_liquidity.saturating_sub(self.locked_reserve);
    }

    /// Add liquidity to pool
    pub fn add_liquidity(&mut self, amount: u64) -> Result<u64> {
        let shares = self.calculate_shares(amount);

        // Check for overflow when adding liquidity
        self.total_liquidity = self.total_liquidity
            .checked_add(amount)
            .ok_or(error!(anchor_lang::error::ErrorCode::AccountDidNotSerialize))?;

        // Check for overflow when adding shares
        self.total_shares = self.total_shares
            .checked_add(shares)
            .ok_or(error!(anchor_lang::error::ErrorCode::AccountDidNotSerialize))?;

        self.available_liquidity = self.total_liquidity.saturating_sub(self.locked_reserve);
        Ok(shares)
    }

    /// Remove liquidity from pool
    pub fn remove_liquidity(&mut self, shares: u64) -> u64 {
        let amount = self.calculate_withdrawal(shares);
        self.total_shares = self.total_shares.saturating_sub(shares);
        self.total_liquidity = self.total_liquidity.saturating_sub(amount);
        self.available_liquidity = self.total_liquidity.saturating_sub(self.locked_reserve);
        amount
    }
}

/// User's LP share position
#[account]
pub struct LpPosition {
    /// User's public key
    pub owner: Pubkey,

    /// Liquidity pool this position belongs to
    pub liquidity_pool: Pubkey,

    /// Number of LP shares owned
    pub shares: u64,

    /// Bump seed for PDA
    pub bump: u8,
}

impl LpPosition {
    pub const LEN: usize = 8 + // discriminator
        32 + // owner
        32 + // liquidity_pool
        8 +  // shares
        1;   // bump
}
