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
    let (favorite_alloc, underdog_alloc, draw_alloc) = if diff > 65 {
        // HUGE FAVORITE: 50/18/32 → 1.16x / 1.94x / 1.56x
        (50, 18, 32)
    } else if diff > 50 {
        // VERY STRONG: 46/23/31 → 1.21x / 1.78x / 1.61x
        (46, 23, 31)
    } else if diff > 35 {
        // STRONG: 42/27/31 → 1.26x / 1.67x / 1.61x
        (42, 27, 31)
    } else if diff > 20 {
        // MODERATE: 38/31/31 → 1.32x / 1.55x / 1.61x
        (38, 31, 31)
    } else if diff > 8 {
        // SLIGHT: 36/33/31 → 1.39x / 1.48x / 1.61x
        (36, 33, 31)
    } else {
        // BALANCED: 34/34/32 → 1.44x / 1.44x / 1.56x
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

    #[test]
    fn test_pseudo_random_seeds() {
        let (home, away, draw) = calculate_pseudo_random_seeds(1, 2, 1);

        // Total should equal SEED_PER_MATCH
        assert_eq!(home + away + draw, SEED_PER_MATCH);

        // All seeds should be non-zero
        assert!(home > 0);
        assert!(away > 0);
        assert!(draw > 0);
    }

    #[test]
    fn test_deterministic_seeding() {
        // Same inputs should produce same outputs
        let (h1, a1, d1) = calculate_pseudo_random_seeds(1, 2, 1);
        let (h2, a2, d2) = calculate_pseudo_random_seeds(1, 2, 1);

        assert_eq!(h1, h2);
        assert_eq!(a1, a2);
        assert_eq!(d1, d2);
    }
}
