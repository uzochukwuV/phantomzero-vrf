use anchor_lang::prelude::*;

/// Pool for a single match with betting on three outcomes
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct MatchPool {
    /// Total tokens bet on HOME_WIN (outcome 1)
    pub home_win_pool: u64,

    /// Total tokens bet on AWAY_WIN (outcome 2)
    pub away_win_pool: u64,

    /// Total tokens bet on DRAW (outcome 3)
    pub draw_pool: u64,

    /// Sum of all three pools
    pub total_pool: u64,
}

impl MatchPool {
    pub fn add_to_pool(&mut self, outcome: u8, amount: u64) {
        match outcome {
            1 => self.home_win_pool += amount,
            2 => self.away_win_pool += amount,
            3 => self.draw_pool += amount,
            _ => panic!("Invalid outcome"),
        }
        self.total_pool += amount;
    }

    pub fn get_pool_amount(&self, outcome: u8) -> u64 {
        match outcome {
            1 => self.home_win_pool,
            2 => self.away_win_pool,
            3 => self.draw_pool,
            _ => 0,
        }
    }
}

/// Locked odds for a match (fixed at seeding time)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct LockedOdds {
    /// Home win odds (scaled by 1e9, e.g., 1.5e9 = 1.5x)
    pub home_odds: u64,

    /// Away win odds (scaled by 1e9)
    pub away_odds: u64,

    /// Draw odds (scaled by 1e9)
    pub draw_odds: u64,

    /// Whether odds have been locked
    pub locked: bool,
}

impl LockedOdds {
    pub fn get_odds(&self, outcome: u8) -> u64 {
        match outcome {
            1 => self.home_odds,
            2 => self.away_odds,
            3 => self.draw_odds,
            _ => 0,
        }
    }
}

/// Match outcome enumeration
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum MatchOutcome {
    Pending = 0,
    HomeWin = 1,
    AwayWin = 2,
    Draw = 3,
}

impl Default for MatchOutcome {
    fn default() -> Self {
        MatchOutcome::Pending
    }
}
