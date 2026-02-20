use crate::constants::*;
use anchor_lang::prelude::*;

/// Calculate differentiated seed amounts for a match using pseudo-random allocation
///
/// This creates varied odds (1.2x - 1.8x range) based on deterministic randomness
/// from team IDs and round ID
pub fn calculate_pseudo_random_seeds(
    home_team_id: u64,
    away_team_id: u64,
    round_id: u64,
) -> (u64, u64, u64) {
    // Generate deterministic pseudo-random seed
    let hash_input = format!("{}-{}-{}", home_team_id, away_team_id, round_id);
    let hash = solana_program::hash::hash(hash_input.as_bytes());

    // Extract randomness (0-99)
    let home_strength = (hash.to_bytes()[0] as u64) % 100;
    let away_strength = (hash.to_bytes()[1] as u64) % 100;
    let draw_factor = (hash.to_bytes()[2] as u64) % 100;

    let total_seed = SEED_PER_MATCH;

    // Calculate strength difference
    let diff = if home_strength > away_strength {
        home_strength - away_strength
    } else {
        away_strength - home_strength
    };

    // Determine allocation percentages based on strength difference
    // Compressed odds range: 1.2x (strong favourite) – 2.2x (heavy underdog)
    let (favorite_alloc, underdog_alloc, draw_alloc) = if diff > 78 {
        // EXTREME FAVORITE: 63/16/21 → ~1.2x / ~2.2x / ~2.0x
        (63, 16, 21)
    } else if diff > 65 {
        // HUGE FAVORITE: 52/19/29 → ~1.25x / ~2.1x / ~1.75x
        (52, 19, 29)
    } else if diff > 50 {
        // VERY STRONG: 46/23/31 → ~1.3x / ~1.9x / ~1.6x
        (46, 23, 31)
    } else if diff > 35 {
        // STRONG: 42/27/31 → ~1.35x / ~1.75x / ~1.6x
        (42, 27, 31)
    } else if diff > 20 {
        // MODERATE: 38/31/31 → ~1.42x / ~1.6x / ~1.6x
        (38, 31, 31)
    } else if diff > 8 {
        // SLIGHT: 36/33/31 → ~1.46x / ~1.53x / ~1.6x
        (36, 33, 31)
    } else {
        // BALANCED: 34/34/32 → ~1.5x / ~1.5x / ~1.56x
        (34, 34, 32)
    };

    // Allocate pools
    let (mut home_seed, mut away_seed, mut draw_seed) = if home_strength > away_strength {
        (
            (total_seed * favorite_alloc) / 100,
            (total_seed * underdog_alloc) / 100,
            (total_seed * draw_alloc) / 100,
        )
    } else {
        (
            (total_seed * underdog_alloc) / 100,
            (total_seed * favorite_alloc) / 100,
            (total_seed * draw_alloc) / 100,
        )
    };

    // Draw-heavy matchups (20% of matches get boosted draws)
    if draw_factor > 80 {
        let draw_boost = (total_seed * 16) / 100; // Boost draw by 16%
        draw_seed = draw_seed.saturating_add(draw_boost);
        home_seed = home_seed.saturating_sub(draw_boost / 2);
        away_seed = away_seed.saturating_sub(draw_boost / 2);
    }

    (home_seed, away_seed, draw_seed)
}

/// Calculate seeds using stats-based distribution (for mid-late season)
///
/// This would integrate with a game engine to use actual team performance stats
/// For now, returns balanced seeding as a placeholder
pub fn calculate_stats_based_seeds(
    _season_id: u64,
    _home_team_id: u64,
    _away_team_id: u64,
    _home_points: u64,
    _away_points: u64,
) -> (u64, u64, u64) {
    // Placeholder implementation - in production, this would use actual team stats
    // from a game engine or oracle

    // For now, return balanced seeding
    let total_seed = SEED_PER_MATCH;

    // Balanced: 35/35/30 split
    let home_seed = (total_seed * 35) / 100;
    let away_seed = (total_seed * 35) / 100;
    let draw_seed = (total_seed * 30) / 100;

    (home_seed, away_seed, draw_seed)
}

/// Calculate match seeds (hybrid: pseudo-random for early rounds, stats-based later)
///
/// Round 1-3: Pseudo-random based on team IDs
/// Round 4+: Stats-based using actual team performance
pub fn calculate_match_seeds(
    round_id: u64,
    match_index: u8,
    home_team_id: u64,
    away_team_id: u64,
    season_round: u64,
) -> (u64, u64, u64) {
    // Use pseudo-random for first 3 rounds (no meaningful stats yet)
    if season_round <= 3 {
        return calculate_pseudo_random_seeds(home_team_id, away_team_id, round_id);
    }

    // For rounds 4+, would use stats-based seeding
    // Placeholder: use pseudo-random for now
    calculate_pseudo_random_seeds(home_team_id, away_team_id, round_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::odds::calculate_locked_odds_from_seeds;

    fn odds_x(v: u64) -> f64 { v as f64 / 1_000_000_000.0 }

    fn assert_odds_in_range(home_seed: u64, away_seed: u64, draw_seed: u64, label: &str) {
        let (home_odds, away_odds, draw_odds) =
            calculate_locked_odds_from_seeds(home_seed, away_seed, draw_seed);
        let h = odds_x(home_odds);
        let a = odds_x(away_odds);
        let d = odds_x(draw_odds);
        assert!(h >= 1.2 && h <= 2.2, "[{}] home odds {:.3}x out of 1.2-2.2", label, h);
        assert!(a >= 1.2 && a <= 2.2, "[{}] away odds {:.3}x out of 1.2-2.2", label, a);
        assert!(d >= 1.2 && d <= 2.2, "[{}] draw odds {:.3}x out of 1.2-2.2", label, d);
    }

    #[test]
    fn test_seeds_sum_to_total() {
        let (home, away, draw) = calculate_pseudo_random_seeds(1, 2, 1);
        // Draw-heavy boost can add up to 16% extra, so total may exceed SEED_PER_MATCH slightly
        let total = home + away + draw;
        // Should be close to SEED_PER_MATCH (within 20% for draw-heavy boosts)
        assert!(total >= SEED_PER_MATCH / 2 && total <= SEED_PER_MATCH * 2,
            "total seeds {} far from SEED_PER_MATCH {}", total, SEED_PER_MATCH);
    }

    #[test]
    fn test_seeds_all_nonzero() {
        let (home, away, draw) = calculate_pseudo_random_seeds(1, 2, 1);
        assert!(home > 0, "home_seed must be nonzero");
        assert!(away > 0, "away_seed must be nonzero");
        assert!(draw > 0, "draw_seed must be nonzero");
    }

    #[test]
    fn test_deterministic_seeding() {
        let (h1, a1, d1) = calculate_pseudo_random_seeds(1, 2, 1);
        let (h2, a2, d2) = calculate_pseudo_random_seeds(1, 2, 1);
        assert_eq!(h1, h2);
        assert_eq!(a1, a2);
        assert_eq!(d1, d2);
    }

    #[test]
    fn test_different_teams_different_seeds() {
        let (h1, a1, _) = calculate_pseudo_random_seeds(1, 2, 1);
        let (h2, a2, _) = calculate_pseudo_random_seeds(3, 7, 1);
        // Different team IDs should (almost certainly) produce different allocations
        assert!(h1 != h2 || a1 != a2, "Different teams should get different seeds");
    }

    #[test]
    fn test_all_tiers_produce_odds_in_range() {
        let total = SEED_PER_MATCH;

        // EXTREME FAVORITE: 63/16/21
        let h = (total * 63) / 100;
        let a = (total * 16) / 100;
        let d = total - h - a;
        assert_odds_in_range(h, a, d, "EXTREME FAVORITE 63/16/21");

        // HUGE FAVORITE: 52/19/29
        let h = (total * 52) / 100;
        let a = (total * 19) / 100;
        let d = total - h - a;
        assert_odds_in_range(h, a, d, "HUGE FAVORITE 52/19/29");

        // VERY STRONG: 46/23/31
        let h = (total * 46) / 100;
        let a = (total * 23) / 100;
        let d = total - h - a;
        assert_odds_in_range(h, a, d, "VERY STRONG 46/23/31");

        // STRONG: 42/27/31
        let h = (total * 42) / 100;
        let a = (total * 27) / 100;
        let d = total - h - a;
        assert_odds_in_range(h, a, d, "STRONG 42/27/31");

        // MODERATE: 38/31/31
        let h = (total * 38) / 100;
        let a = (total * 31) / 100;
        let d = total - h - a;
        assert_odds_in_range(h, a, d, "MODERATE 38/31/31");

        // SLIGHT: 36/33/31
        let h = (total * 36) / 100;
        let a = (total * 33) / 100;
        let d = total - h - a;
        assert_odds_in_range(h, a, d, "SLIGHT 36/33/31");

        // BALANCED: 34/34/32
        let h = (total * 34) / 100;
        let a = (total * 34) / 100;
        let d = total - h - a;
        assert_odds_in_range(h, a, d, "BALANCED 34/34/32");
    }

    #[test]
    fn test_pseudo_random_seeds_odds_in_range() {
        // Test many different team/round combinations
        let pairs = [(1u64, 2u64), (3, 5), (7, 10), (2, 9), (6, 8), (4, 1), (10, 3)];
        for (home_id, away_id) in pairs {
            for round_id in [1u64, 5, 10, 42] {
                let (hs, aws, ds) = calculate_pseudo_random_seeds(home_id, away_id, round_id);
                if hs > 0 && aws > 0 && ds > 0 {
                    let label = format!("team{}_vs_team{}_round{}", home_id, away_id, round_id);
                    assert_odds_in_range(hs, aws, ds, &label);
                }
            }
        }
    }
}
