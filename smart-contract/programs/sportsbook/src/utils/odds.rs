use crate::constants::*;
use crate::state::MatchPool;

/// Compress raw parimutuel odds to target 1.2x - 2.2x range
///
/// Maps raw odds (1.8x - 5.5x raw) to compressed range (1.2x - 2.2x)
/// Strong favorites compress to near 1.2x; heavy underdogs to near 2.2x
pub fn compress_odds(raw_odds: u64) -> u64 {
    // Floor: 1.2x — even heavy favorites must pay something
    if raw_odds < RAW_ODDS_MIN {
        return MIN_COMPRESSED_ODDS;
    }

    // Ceiling: 2.2x — cap huge underdogs
    if raw_odds > RAW_ODDS_MAX {
        return MAX_COMPRESSED_ODDS;
    }

    // Linear compression formula:
    // compressed = minOdds + (raw - minRaw) × (maxOdds - minOdds) / (maxRaw - minRaw)
    // compressed = 1.2 + (raw - 1.8) × (2.2 - 1.2) / (5.5 - 1.8)
    // compressed = 1.2 + (raw - 1.8) × 1.0 / 3.7
    // compressed = 1.2 + (raw - 1.8) × 0.2703

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
    // Use u128 to prevent overflow: SEED_PER_MATCH * VIRTUAL_LIQUIDITY_MULTIPLIER can exceed u64::MAX
    let virtual_liquidity = ((SEED_PER_MATCH as u128)
        .checked_mul(VIRTUAL_LIQUIDITY_MULTIPLIER as u128)
        .unwrap_or(0))
        .min(u64::MAX as u128) as u64;

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

    fn odds_to_x(odds: u64) -> f64 {
        odds as f64 / 1_000_000_000.0
    }

    #[test]
    fn test_compress_odds_floors_at_1_2x() {
        // Raw below RAW_ODDS_MIN → clamp to 1.2x
        assert_eq!(compress_odds(1_000_000_000), MIN_COMPRESSED_ODDS); // 1.0x raw
        assert_eq!(compress_odds(1_500_000_000), MIN_COMPRESSED_ODDS); // 1.5x raw
        assert_eq!(compress_odds(RAW_ODDS_MIN - 1), MIN_COMPRESSED_ODDS);
        let result = odds_to_x(compress_odds(RAW_ODDS_MIN - 1));
        assert!((result - 1.2).abs() < 0.01, "Expected ~1.2x, got {:.3}x", result);
    }

    #[test]
    fn test_compress_odds_caps_at_2_2x() {
        // Raw above RAW_ODDS_MAX → clamp to 2.2x
        assert_eq!(compress_odds(10_000_000_000), MAX_COMPRESSED_ODDS); // 10x raw
        assert_eq!(compress_odds(RAW_ODDS_MAX + 1), MAX_COMPRESSED_ODDS);
        let result = odds_to_x(compress_odds(RAW_ODDS_MAX + 1));
        assert!((result - 2.2).abs() < 0.01, "Expected ~2.2x, got {:.3}x", result);
    }

    #[test]
    fn test_compress_odds_range_is_1_2_to_2_2() {
        // All raw odds in [RAW_ODDS_MIN, RAW_ODDS_MAX] must compress to [1.2x, 2.2x]
        let test_raws = [
            RAW_ODDS_MIN,
            2_000_000_000,  // 2.0x
            3_000_000_000,  // 3.0x
            4_000_000_000,  // 4.0x
            5_000_000_000,  // 5.0x
            RAW_ODDS_MAX,
        ];
        for raw in test_raws {
            let c = compress_odds(raw);
            let x = odds_to_x(c);
            assert!(
                x >= 1.2 && x <= 2.2,
                "raw={:.2}x compressed to {:.3}x — out of 1.2-2.2 range",
                raw as f64 / 1e9,
                x
            );
        }
    }

    #[test]
    fn test_compress_odds_monotonically_increasing() {
        // Higher raw odds → higher compressed odds
        let raws = [1_800_000_000u64, 2_500_000_000, 3_500_000_000, 4_500_000_000, 5_500_000_000];
        for i in 0..raws.len() - 1 {
            let c_lo = compress_odds(raws[i]);
            let c_hi = compress_odds(raws[i + 1]);
            assert!(
                c_hi >= c_lo,
                "Compression not monotone: raw {:.1}x → {:.3}x, raw {:.1}x → {:.3}x",
                raws[i] as f64 / 1e9, c_lo as f64 / 1e9,
                raws[i+1] as f64 / 1e9, c_hi as f64 / 1e9,
            );
        }
    }

    #[test]
    fn test_locked_odds_in_range_default_seeds() {
        let (home, away, draw) = calculate_locked_odds_from_seeds(
            SEED_HOME_POOL,
            SEED_AWAY_POOL,
            SEED_DRAW_POOL,
        );
        let h = odds_to_x(home);
        let a = odds_to_x(away);
        let d = odds_to_x(draw);
        assert!(h >= 1.2 && h <= 2.2, "home odds {:.3}x out of range", h);
        assert!(a >= 1.2 && a <= 2.2, "away odds {:.3}x out of range", a);
        assert!(d >= 1.2 && d <= 2.2, "draw odds {:.3}x out of range", d);
    }

    #[test]
    fn test_locked_odds_extreme_favorite() {
        // Extreme home favourite: 63/16/21 split
        let total = SEED_PER_MATCH;
        let home_seed = (total * 63) / 100;
        let away_seed = (total * 16) / 100;
        let draw_seed = total - home_seed - away_seed;

        let (home, away, draw) = calculate_locked_odds_from_seeds(home_seed, away_seed, draw_seed);
        let h = odds_to_x(home);
        let a = odds_to_x(away);
        let d = odds_to_x(draw);

        // Extreme favorite → near floor (1.2x); extreme underdog → near ceiling (2.2x)
        assert!(h >= 1.2 && h <= 1.35, "home should be near 1.2x, got {:.3}x", h);
        assert!(a >= 1.9 && a <= 2.2,  "away should be near 2.2x, got {:.3}x", a);
        assert!(d >= 1.2 && d <= 2.2,  "draw should be in range, got {:.3}x", d);
    }

    #[test]
    fn test_locked_odds_balanced_match() {
        // Balanced: 34/34/32 split
        let total = SEED_PER_MATCH;
        let home_seed = (total * 34) / 100;
        let away_seed = (total * 34) / 100;
        let draw_seed = total - home_seed - away_seed;

        let (home, away, draw) = calculate_locked_odds_from_seeds(home_seed, away_seed, draw_seed);
        let h = odds_to_x(home);
        let a = odds_to_x(away);
        let d = odds_to_x(draw);

        // Balanced match → both sides similar odds, around 1.4-1.6x
        assert!(h >= 1.2 && h <= 2.2, "home out of range: {:.3}x", h);
        assert!(a >= 1.2 && a <= 2.2, "away out of range: {:.3}x", a);
        assert!(d >= 1.2 && d <= 2.2, "draw out of range: {:.3}x", d);
        // Home and away should be close for balanced match
        let diff = (h - a).abs();
        assert!(diff < 0.15, "Balanced match: home {:.3}x vs away {:.3}x too different", h, a);
    }

    #[test]
    fn test_locked_odds_empty_pool_fallback() {
        // Empty pools should return fallback 1.5x
        let (home, away, draw) = calculate_locked_odds_from_seeds(0, 0, 0);
        assert_eq!(home, 1_500_000_000);
        assert_eq!(away, 1_500_000_000);
        assert_eq!(draw, 1_500_000_000);
    }
}
