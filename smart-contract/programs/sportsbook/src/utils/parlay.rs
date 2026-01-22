use crate::constants::*;
use crate::state::RoundAccounting;
use crate::utils::odds::calculate_pool_imbalance;

/// Get base parlay multiplier based on number of matches
///
/// Linear progression: 1.0x (1 match) to 1.25x (10 matches)
pub fn get_base_parlay_multiplier(num_matches: u8) -> u64 {
    match num_matches {
        1 => PARLAY_MULTIPLIER_1_MATCH,
        2 => PARLAY_MULTIPLIER_2_MATCHES,
        3 => PARLAY_MULTIPLIER_3_MATCHES,
        4 => PARLAY_MULTIPLIER_4_MATCHES,
        5 => PARLAY_MULTIPLIER_5_MATCHES,
        6 => PARLAY_MULTIPLIER_6_MATCHES,
        7 => PARLAY_MULTIPLIER_7_MATCHES,
        8 => PARLAY_MULTIPLIER_8_MATCHES,
        9 => PARLAY_MULTIPLIER_9_MATCHES,
        _ => PARLAY_MULTIPLIER_10_MATCHES, // 10+ capped at 1.25x
    }
}

/// Get count-based parlay multiplier (PRIMARY FOMO mechanism)
///
/// Returns multiplier based on parlay index in current round
pub fn get_parlay_multiplier_by_count(parlay_index: u64) -> u64 {
    if parlay_index < COUNT_TIER_1 {
        COUNT_MULT_TIER_1 // 2.5x (first 10)
    } else if parlay_index < COUNT_TIER_2 {
        COUNT_MULT_TIER_2 // 2.2x (next 10)
    } else if parlay_index < COUNT_TIER_3 {
        COUNT_MULT_TIER_3 // 1.9x (next 10)
    } else if parlay_index < COUNT_TIER_4 {
        COUNT_MULT_TIER_4 // 1.6x (next 10)
    } else {
        COUNT_MULT_TIER_5 // 1.3x (41+)
    }
}

/// Get reserve-based decay factor (SECONDARY safety valve)
///
/// Higher locked reserve = lower multipliers (capital protection)
/// In unified LP model, no reserve decay - LP pool manages all risk
pub fn get_reserve_decay_factor(_locked_reserve: u64) -> u64 {
    TIER_1_DECAY // 100% (no decay in unified LP model)
}

/// Calculate liquidity-aware parlay multiplier (DYNAMIC)
///
/// Combines 3 layers:
/// 1. Base multiplier from number of matches
/// 2. Pool imbalance gating (economic protection)
/// 3. Reserve-based decay (safety valve)
pub fn calculate_parlay_multiplier_dynamic(
    round_accounting: &RoundAccounting,
    match_indices: &[u8],
    num_legs: u8,
) -> u64 {
    // Single bets always get 1.0x
    if num_legs == 1 {
        return ODDS_SCALE;
    }

    // LAYER 1: Base multiplier based on number of matches
    let base_multiplier = get_base_parlay_multiplier(num_legs);

    // LAYER 2: Pool imbalance gating (ECONOMIC PROTECTION)
    let mut total_imbalance = 0u64;
    for &match_index in match_indices.iter() {
        let pool = &round_accounting.match_pools[match_index as usize];
        let imbalance = calculate_pool_imbalance(pool);
        total_imbalance = total_imbalance.saturating_add(imbalance);
    }
    let avg_imbalance = total_imbalance / (match_indices.len() as u64);

    // If pools are balanced, reduce to minimum regardless of tier
    if avg_imbalance < MIN_IMBALANCE_FOR_FULL_BONUS {
        return MIN_PARLAY_MULTIPLIER; // 1.1x
    }

    // LAYER 3: Reserve-based decay (in unified LP model, this is 100%)
    let decay_factor = get_reserve_decay_factor(0);
    let final_multiplier = (base_multiplier as u128)
        .checked_mul(decay_factor as u128)
        .unwrap_or(0)
        .checked_div(BPS_DENOMINATOR as u128)
        .unwrap_or(0) as u64;

    // Never go below minimum
    if final_multiplier < MIN_PARLAY_MULTIPLIER {
        MIN_PARLAY_MULTIPLIER
    } else {
        final_multiplier
    }
}

/// Calculate odds-weighted allocations for parlay bets
///
/// Allocates tokens such that each match contributes equally to target payout
/// Returns (allocations, total_allocated, lp_borrowed)
pub fn calculate_odds_weighted_allocations(
    round_accounting: &RoundAccounting,
    match_indices: &[u8],
    outcomes: &[u8],
    amount_after_fee: u64,
    parlay_multiplier: u64,
) -> Result<(Vec<u64>, u64, u64), &'static str> {
    let mut allocations = Vec::with_capacity(match_indices.len());

    // Step 1: Calculate target final payout
    // Base payout = product of all odds
    let mut base_payout = amount_after_fee;
    for (i, &match_index) in match_indices.iter().enumerate() {
        let odds = &round_accounting.locked_odds[match_index as usize];
        if !odds.locked {
            return Err("Odds not locked - seed round first");
        }

        // Get odds for predicted outcome
        let match_odds = odds.get_odds(outcomes[i]);

        // Multiply: base_payout = base_payout × match_odds / ODDS_SCALE
        base_payout = (base_payout as u128)
            .checked_mul(match_odds as u128)
            .ok_or("Parlay calculation overflow")?
            .checked_div(ODDS_SCALE as u128)
            .ok_or("Division error")? as u64;
    }

    // Apply parlay multiplier
    let target_payout = (base_payout as u128)
        .checked_mul(parlay_multiplier as u128)
        .ok_or("Parlay multiplier overflow")?
        .checked_div(ODDS_SCALE as u128)
        .ok_or("Division error")? as u64;

    // Step 2: Calculate per-match contribution (equal contribution)
    let per_match_contribution = target_payout / (match_indices.len() as u64);

    // Step 3: Calculate required allocation for each match (working backwards)
    let mut total_allocated = 0u64;
    for (i, &match_index) in match_indices.iter().enumerate() {
        let odds = &round_accounting.locked_odds[match_index as usize];
        let match_odds = odds.get_odds(outcomes[i]);

        // Calculate: allocation = per_match_contribution / match_odds
        let allocation = (per_match_contribution as u128)
            .checked_mul(ODDS_SCALE as u128)
            .ok_or("Allocation calculation overflow")?
            .checked_div(match_odds as u128)
            .ok_or("Division error")? as u64;

        allocations.push(allocation);
        total_allocated = total_allocated.saturating_add(allocation);
    }

    // Step 4: Calculate LP borrowing needed
    let lp_borrowed = if total_allocated > amount_after_fee {
        total_allocated - amount_after_fee
    } else {
        0
    };

    Ok((allocations, total_allocated, lp_borrowed))
}

/// Calculate maximum possible payout for a bet
///
/// Used to check if LP pool can cover potential winnings
pub fn calculate_max_payout(amount: u64, num_matches: u8, parlay_multiplier: u64) -> u64 {
    // Pessimistic estimate: assume best case odds (2x per match)
    let max_base_payout = amount.saturating_mul(2u64.saturating_pow(num_matches as u32));

    // Apply parlay multiplier
    let max_final_payout = (max_base_payout as u128)
        .checked_mul(parlay_multiplier as u128)
        .unwrap_or(0)
        .checked_div(ODDS_SCALE as u128)
        .unwrap_or(0) as u64;

    // Apply per-bet cap
    if max_final_payout > MAX_PAYOUT_PER_BET {
        MAX_PAYOUT_PER_BET
    } else {
        max_final_payout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_parlay_multiplier() {
        assert_eq!(get_base_parlay_multiplier(1), PARLAY_MULTIPLIER_1_MATCH);
        assert_eq!(get_base_parlay_multiplier(2), PARLAY_MULTIPLIER_2_MATCHES);
        assert_eq!(get_base_parlay_multiplier(10), PARLAY_MULTIPLIER_10_MATCHES);
    }

    #[test]
    fn test_count_based_multiplier() {
        assert_eq!(get_parlay_multiplier_by_count(0), COUNT_MULT_TIER_1);
        assert_eq!(get_parlay_multiplier_by_count(9), COUNT_MULT_TIER_1);
        assert_eq!(get_parlay_multiplier_by_count(10), COUNT_MULT_TIER_2);
        assert_eq!(get_parlay_multiplier_by_count(50), COUNT_MULT_TIER_5);
    }
}
