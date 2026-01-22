use crate::constants::*;
use crate::state::MatchPool;

/// Compress raw parimutuel odds to target 1.25x - 1.95x range
///
/// This creates profitable odds for the LP while keeping betting attractive
/// Maps raw odds (1.8x - 5.5x) to compressed range (1.25x - 1.95x)
pub fn compress_odds(raw_odds: u64) -> u64 {
    // Minimum odds: 1.25x (even huge favorites must pay something)
    if raw_odds < RAW_ODDS_MIN {
        return MIN_COMPRESSED_ODDS;
    }

    // Maximum odds: 1.95x (cap huge underdogs)
    if raw_odds > RAW_ODDS_MAX {
        return MAX_COMPRESSED_ODDS;
    }

    // Linear compression formula:
    // compressed = minOdds + (raw - minRaw) × (maxOdds - minOdds) / (maxRaw - minRaw)
    // compressed = 1.25 + (raw - 1.8) × (1.95 - 1.25) / (5.5 - 1.8)
    // compressed = 1.25 + (raw - 1.8) × 0.7 / 3.7
    // compressed = 1.25 + (raw - 1.8) × 0.189

    let excess = raw_odds.saturating_sub(RAW_ODDS_MIN);
    let range = RAW_ODDS_MAX - RAW_ODDS_MIN; // 3.7e9
    let target_range = MAX_COMPRESSED_ODDS - MIN_COMPRESSED_ODDS; // 0.7e9

    let scaled_excess = (excess as u128)
        .checked_mul(target_range as u128)
        .unwrap_or(0)
        .checked_div(range as u128)
        .unwrap_or(0) as u64;

    MIN_COMPRESSED_ODDS + scaled_excess
}

/// Calculate locked odds from initial seed pools
///
/// This is called once at seeding time to lock odds for the entire round
/// Everyone gets paid at these fixed odds, making accounting exact
pub fn calculate_locked_odds_from_seeds(
    home_seed: u64,
    away_seed: u64,
    draw_seed: u64,
) -> (u64, u64, u64) {
    let total_pool = home_seed + away_seed + draw_seed;

    if total_pool == 0 {
        // Fallback: equal odds
        return (
            1_500_000_000, // 1.5x
            1_500_000_000,
            1_500_000_000,
        );
    }

    // Calculate raw parimutuel odds
    let raw_home_odds = (total_pool as u128)
        .checked_mul(ODDS_SCALE as u128)
        .unwrap_or(0)
        .checked_div(home_seed as u128)
        .unwrap_or(ODDS_SCALE as u128) as u64;

    let raw_away_odds = (total_pool as u128)
        .checked_mul(ODDS_SCALE as u128)
        .unwrap_or(0)
        .checked_div(away_seed as u128)
        .unwrap_or(ODDS_SCALE as u128) as u64;

    let raw_draw_odds = (total_pool as u128)
        .checked_mul(ODDS_SCALE as u128)
        .unwrap_or(0)
        .checked_div(draw_seed as u128)
        .unwrap_or(ODDS_SCALE as u128) as u64;

    // Compress to target range
    (
        compress_odds(raw_home_odds),
        compress_odds(raw_away_odds),
        compress_odds(raw_draw_odds),
    )
}

/// Calculate market odds with virtual liquidity dampening
///
/// Used for previewing odds before betting
pub fn calculate_market_odds(pool: &MatchPool, outcome: u8) -> u64 {
    let winning_pool = pool.get_pool_amount(outcome);
    let total_pool = pool.total_pool;

    if winning_pool == 0 {
        return 3 * ODDS_SCALE; // Fallback: fair VRF odds (33.33% = 3.0x)
    }

    // Apply virtual liquidity to dampen price impact
    let virtual_liquidity = SEED_PER_MATCH
        .checked_mul(VIRTUAL_LIQUIDITY_MULTIPLIER)
        .unwrap_or(0);

    // Add virtual liquidity proportionally (33.33% per outcome)
    let virtual_winning_pool = winning_pool + (virtual_liquidity / 3);
    let virtual_total_pool = total_pool + virtual_liquidity;

    // Calculate dampened odds
    (virtual_total_pool as u128)
        .checked_mul(ODDS_SCALE as u128)
        .unwrap_or(0)
        .checked_div(virtual_winning_pool as u128)
        .unwrap_or(ODDS_SCALE as u128) as u64
}

/// Calculate pool imbalance (measures dominance of largest pool)
///
/// Returns imbalance in basis points (0-10000, where 10000 = 100%)
pub fn calculate_pool_imbalance(pool: &MatchPool) -> u64 {
    if pool.total_pool == 0 {
        return 0;
    }

    // Find max pool
    let max_pool = pool
        .home_win_pool
        .max(pool.away_win_pool)
        .max(pool.draw_pool);

    // Return as basis points
    (max_pool as u128)
        .checked_mul(BPS_DENOMINATOR as u128)
        .unwrap_or(0)
        .checked_div(pool.total_pool as u128)
        .unwrap_or(0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_odds() {
        // Test minimum compression
        assert_eq!(compress_odds(1_000_000_000), MIN_COMPRESSED_ODDS);

        // Test maximum compression
        assert_eq!(compress_odds(10_000_000_000), MAX_COMPRESSED_ODDS);

        // Test mid-range
        let mid_raw = 3_650_000_000; // 3.65x raw
        let compressed = compress_odds(mid_raw);
        assert!(compressed > MIN_COMPRESSED_ODDS);
        assert!(compressed < MAX_COMPRESSED_ODDS);
    }

    #[test]
    fn test_calculate_locked_odds() {
        let (home, away, draw) = calculate_locked_odds_from_seeds(
            SEED_HOME_POOL,
            SEED_AWAY_POOL,
            SEED_DRAW_POOL,
        );

        // All odds should be in valid range
        assert!(home >= MIN_COMPRESSED_ODDS && home <= MAX_COMPRESSED_ODDS);
        assert!(away >= MIN_COMPRESSED_ODDS && away <= MAX_COMPRESSED_ODDS);
        assert!(draw >= MIN_COMPRESSED_ODDS && draw <= MAX_COMPRESSED_ODDS);
    }
}
