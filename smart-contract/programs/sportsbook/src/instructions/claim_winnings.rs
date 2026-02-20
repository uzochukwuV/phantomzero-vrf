use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{BettingPool, RoundAccounting, Bet, MatchOutcome};
use crate::errors::SportsbookError;
use crate::constants::*;

#[derive(Accounts)]
#[instruction(bet_id: u64)]
pub struct ClaimWinnings<'info> {
    #[account(mut)]
    pub betting_pool: Account<'info, BettingPool>,

    #[account(
        mut,
        seeds = [b"round", betting_pool.key().as_ref(), bet.round_id.to_le_bytes().as_ref()],
        bump = round_accounting.bump,
        constraint = round_accounting.settled @ SportsbookError::RoundNotSettled,
    )]
    pub round_accounting: Account<'info, RoundAccounting>,

    #[account(
        mut,
        seeds = [b"bet", betting_pool.key().as_ref(), bet_id.to_le_bytes().as_ref()],
        bump = bet.bump,
        constraint = !bet.claimed @ SportsbookError::BetAlreadyClaimed,
    )]
    pub bet: Account<'info, Bet>,

    /// Betting pool's token account (protocol provides all liquidity)
    #[account(mut)]
    pub betting_pool_token_account: Account<'info, TokenAccount>,

    /// Bettor's token account (receives winnings or 90% if bounty claim)
    /// CHECK: Verified against bet.bettor
    #[account(mut)]
    pub bettor_token_account: UncheckedAccount<'info>,

    /// Claimer (can be bettor or bounty hunter after 24h)
    /// If claiming within 24h, must be the bettor
    /// If claiming after 24h, can be anyone (receives 10% bounty)
    #[account(mut)]
    pub claimer: Signer<'info>,

    /// Claimer's token account (receives 10% bounty if third-party claim)
    /// CHECK: Only used for bounty claims after deadline
    #[account(mut)]
    pub claimer_token_account: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<ClaimWinnings>,
    bet_id: u64,
    min_payout: u64,
) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Extract account infos and keys BEFORE mutable borrows
    let betting_pool_info = ctx.accounts.betting_pool.to_account_info();
    let betting_pool_bump = ctx.accounts.betting_pool.bump;

    // Calculate claim deadline: 24 hours after round settlement
    // 86400 seconds = 24 hours
    let claim_deadline = ctx.accounts.round_accounting.round_end_time + 86400;

    // Update bet's claim_deadline if not set yet
    if ctx.accounts.bet.claim_deadline == 0 {
        ctx.accounts.bet.claim_deadline = claim_deadline;
    }

    // Check claim window and determine if this is a bounty claim
    let is_bettor = ctx.accounts.claimer.key() == ctx.accounts.bet.bettor;
    let is_bounty_claim = current_time > claim_deadline && !is_bettor;

    // If within 24h window, only bettor can claim
    if current_time <= claim_deadline {
        require!(is_bettor, SportsbookError::NotBettor);
    }

    // Calculate if bet won and payout amount
    let (won, base_payout, final_payout) = calculate_bet_payout(&ctx.accounts.bet, &ctx.accounts.round_accounting)?;

    // Slippage protection
    require!(
        final_payout >= min_payout,
        SportsbookError::PayoutBelowMinimum
    );

    // Mark as claimed and settled
    ctx.accounts.bet.claimed = true;
    ctx.accounts.bet.settled = true;

    if won && final_payout > 0 {
        // Check per-round payout cap
        require!(
            ctx.accounts.round_accounting.total_paid_out + final_payout <= MAX_ROUND_PAYOUTS,
            SportsbookError::RoundPayoutLimitReached
        );

        // Update accounting
        ctx.accounts.round_accounting.total_claimed += final_payout;
        ctx.accounts.round_accounting.total_paid_out += final_payout;

        // Calculate bounty split if applicable
        let (bettor_amount, bounty_amount) = if is_bounty_claim {
            // 90% to bettor, 10% to claimer
            let bounty = (final_payout as u128)
                .checked_mul(1000)  // 10% = 1000 / 10000
                .ok_or(SportsbookError::CalculationOverflow)?
                .checked_div(10000)
                .ok_or(SportsbookError::CalculationOverflow)? as u64;
            let bettor_share = final_payout.saturating_sub(bounty);

            // Record bounty claimer
            ctx.accounts.bet.bounty_claimer = Some(ctx.accounts.claimer.key());

            msg!("Bounty claim by {}: 10% bounty = {}", ctx.accounts.claimer.key(), bounty);
            (bettor_share, bounty)
        } else {
            // Bettor claims within 24h, gets 100%
            (final_payout, 0)
        };

        let betting_pool_balance = ctx.accounts.betting_pool_token_account.amount;

        // Ensure protocol has enough to pay (should always be true)
        require!(
            betting_pool_balance >= final_payout,
            SportsbookError::InsufficientProtocolLiquidity
        );

        let seeds = &[b"betting_pool".as_ref(), &[betting_pool_bump]];
        let signer = &[&seeds[..]];

        // Pay bettor their share
        let cpi_accounts = Transfer {
            from: ctx.accounts.betting_pool_token_account.to_account_info(),
            to: ctx.accounts.bettor_token_account.to_account_info(),
            authority: betting_pool_info.clone(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, bettor_amount)?;

        // Pay bounty to claimer if applicable
        if bounty_amount > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.betting_pool_token_account.to_account_info(),
                to: ctx.accounts.claimer_token_account.to_account_info(),
                authority: betting_pool_info.clone(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
            token::transfer(cpi_ctx, bounty_amount)?;
        }

        msg!("Bet {} won! Paid out {} tokens (bettor: {}, bounty: {})",
             bet_id, final_payout, bettor_amount, bounty_amount);
        msg!("Base payout: {}, Parlay multiplier: {}", base_payout, ctx.accounts.bet.locked_multiplier);
    } else {
        msg!("Bet {} lost", bet_id);
    }

    Ok(())
}

/// Calculate bet payout with parlay multiplier
fn calculate_bet_payout(
    bet: &Bet,
    round_accounting: &RoundAccounting,
) -> Result<(bool, u64, u64)> {
    let mut all_correct = true;
    let mut total_base_payout = 0u64;

    let predictions = bet.get_predictions();

    for prediction in predictions {
        let match_result = &round_accounting.match_results[prediction.match_index as usize];
        let locked_odds = &round_accounting.locked_odds[prediction.match_index as usize];

        // Check if prediction is correct
        let predicted_outcome = match prediction.predicted_outcome {
            1 => MatchOutcome::HomeWin,
            2 => MatchOutcome::AwayWin,
            3 => MatchOutcome::Draw,
            _ => MatchOutcome::Pending,
        };

        if *match_result != predicted_outcome {
            all_correct = false;
            break;
        }

        // Use locked odds for payout calculation
        require!(locked_odds.locked, SportsbookError::OddsNotLocked);

        let odds = locked_odds.get_odds(prediction.predicted_outcome);

        // Simple multiplication: amount × locked odds
        let match_payout = (prediction.amount_in_pool as u128)
            .checked_mul(odds as u128)
            .ok_or(SportsbookError::CalculationOverflow)?
            .checked_div(ODDS_SCALE as u128)
            .ok_or(SportsbookError::CalculationOverflow)? as u64;

        total_base_payout += match_payout;
    }

    if !all_correct {
        return Ok((false, 0, 0));
    }

    // Apply locked parlay multiplier
    let total_final_payout = (total_base_payout as u128)
        .checked_mul(bet.locked_multiplier as u128)
        .ok_or(SportsbookError::CalculationOverflow)?
        .checked_div(ODDS_SCALE as u128)
        .ok_or(SportsbookError::CalculationOverflow)? as u64;

    // Cap maximum payout per bet
    let capped_payout = if total_final_payout > MAX_PAYOUT_PER_BET {
        MAX_PAYOUT_PER_BET
    } else {
        total_final_payout
    };

    Ok((true, total_base_payout, capped_payout))
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests for win/loss determination and claim payout logic
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Bet, RoundAccounting, MatchPool, LockedOdds, MatchOutcome, Prediction};
    use crate::constants::*;
    use anchor_lang::prelude::Pubkey;

    const SCALE: u64 = ODDS_SCALE;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn make_prediction(match_index: u8, outcome: u8, amount: u64) -> Prediction {
        Prediction { match_index, predicted_outcome: outcome, amount_in_pool: amount }
    }

    /// Build a Bet with num_predictions slots filled from the slice.
    fn make_bet(preds: &[Prediction], multiplier: u64) -> Bet {
        let mut arr = [Prediction { match_index: 0, predicted_outcome: 0, amount_in_pool: 0 }; 10];
        for (i, p) in preds.iter().enumerate() {
            arr[i] = *p;
        }
        Bet {
            bettor: Pubkey::default(),
            round_id: 0,
            bet_id: 0,
            amount: 0,
            amount_after_fee: 0,
            allocated_amount: 0,
            bonus: 0,
            locked_multiplier: multiplier,
            num_predictions: preds.len() as u8,
            predictions: arr,
            settled: false,
            claimed: false,
            claim_deadline: 0,
            bounty_claimer: None,
            bump: 0,
        }
    }

    /// Build a RoundAccounting with per-match results and locked odds.
    /// results: length-10 array of MatchOutcome; odds: (home, away, draw) per match.
    fn make_round(
        results: [MatchOutcome; 10],
        odds: [(u64, u64, u64); 10],
    ) -> RoundAccounting {
        let mut locked = [LockedOdds::default(); 10];
        for i in 0..10 {
            locked[i] = LockedOdds {
                home_odds: odds[i].0,
                away_odds: odds[i].1,
                draw_odds: odds[i].2,
                locked: true,
            };
        }
        RoundAccounting {
            round_id: 1,
            betting_pool: Pubkey::default(),
            match_pools: [MatchPool::default(); 10],
            locked_odds: locked,
            match_results: results,
            total_bet_volume: 0,
            total_winning_pool: 0,
            total_losing_pool: 0,
            total_reserved_for_winners: 0,
            total_claimed: 0,
            total_paid_out: 0,
            protocol_fee_collected: 0,
            protocol_revenue_share: 0,
            season_revenue_share: 0,
            revenue_distributed: false,
            protocol_seed_amount: 0,
            seeded: true,
            total_user_deposits: 0,
            parlay_count: 0,
            round_start_time: 0,
            round_end_time: 0,
            settled: true,
            bump: 0,
        }
    }

    fn default_results() -> [MatchOutcome; 10] {
        [MatchOutcome::Pending; 10]
    }

    fn default_odds(home: u64, away: u64, draw: u64) -> [(u64, u64, u64); 10] {
        [(home, away, draw); 10]
    }

    // ── single bet: win ───────────────────────────────────────────────────────

    #[test]
    fn test_single_bet_home_win_correct() {
        // Odds: home = 1.5x, stake = 1000 tokens
        let odds = default_odds(1_500_000_000, 2_000_000_000, 1_800_000_000);
        let mut results = default_results();
        results[0] = MatchOutcome::HomeWin;

        let round = make_round(results, odds);
        let bet = make_bet(&[make_prediction(0, 1, 1_000_000_000)], SCALE); // 1x parlay

        let (won, base_payout, final_payout) = calculate_bet_payout(&bet, &round).unwrap();

        assert!(won, "Should have won");
        // payout = stake × odds = 1e9 × 1.5e9 / 1e9 = 1.5e9
        assert_eq!(base_payout, 1_500_000_000);
        assert_eq!(final_payout, 1_500_000_000); // 1x multiplier
    }

    #[test]
    fn test_single_bet_away_win_correct() {
        let odds = default_odds(1_400_000_000, 2_200_000_000, 1_700_000_000);
        let mut results = default_results();
        results[3] = MatchOutcome::AwayWin;

        let round = make_round(results, odds);
        let bet = make_bet(&[make_prediction(3, 2, 500_000_000)], SCALE);

        let (won, _, final_payout) = calculate_bet_payout(&bet, &round).unwrap();
        assert!(won);
        // payout = 5e8 × 2.2e9 / 1e9 = 1.1e9
        assert_eq!(final_payout, 1_100_000_000);
    }

    #[test]
    fn test_single_bet_draw_correct() {
        let odds = default_odds(1_300_000_000, 1_900_000_000, 1_700_000_000);
        let mut results = default_results();
        results[7] = MatchOutcome::Draw;

        let round = make_round(results, odds);
        let bet = make_bet(&[make_prediction(7, 3, 1_000_000_000)], SCALE);

        let (won, _, final_payout) = calculate_bet_payout(&bet, &round).unwrap();
        assert!(won);
        assert_eq!(final_payout, 1_700_000_000); // 1.7x
    }

    // ── single bet: loss ──────────────────────────────────────────────────────

    #[test]
    fn test_single_bet_home_win_wrong_outcome() {
        let odds = default_odds(1_500_000_000, 2_000_000_000, 1_800_000_000);
        let mut results = default_results();
        results[0] = MatchOutcome::AwayWin; // result differs from prediction

        let round = make_round(results, odds);
        let bet = make_bet(&[make_prediction(0, 1, 1_000_000_000)], SCALE); // predicted HOME

        let (won, base, final_p) = calculate_bet_payout(&bet, &round).unwrap();
        assert!(!won, "Should have lost");
        assert_eq!(base, 0);
        assert_eq!(final_p, 0);
    }

    #[test]
    fn test_single_bet_draw_but_home_won() {
        let odds = default_odds(1_200_000_000, 2_200_000_000, 1_600_000_000);
        let mut results = default_results();
        results[2] = MatchOutcome::HomeWin;

        let round = make_round(results, odds);
        let bet = make_bet(&[make_prediction(2, 3, 800_000_000)], SCALE); // predicted DRAW

        let (won, _, _) = calculate_bet_payout(&bet, &round).unwrap();
        assert!(!won);
    }

    // ── parlay bets ───────────────────────────────────────────────────────────

    #[test]
    fn test_parlay_2_match_both_correct() {
        // Match 0 → HomeWin 1.5x, Match 1 → AwayWin 2.0x
        // Stake split: 600 on match-0, 400 on match-1
        // Base = 600×1.5 + 400×2.0 = 900 + 800 = 1700
        // Parlay mult = 1.05x → final = 1700 × 1.05 = 1785
        let odds = default_odds(1_500_000_000, 2_000_000_000, 1_800_000_000);
        let mut results = default_results();
        results[0] = MatchOutcome::HomeWin;
        results[1] = MatchOutcome::AwayWin;

        let round = make_round(results, odds);
        let parlay_mult = 1_050_000_000u64; // 1.05x
        let bet = make_bet(&[
            make_prediction(0, 1, 600_000_000),
            make_prediction(1, 2, 400_000_000),
        ], parlay_mult);

        let (won, base, final_p) = calculate_bet_payout(&bet, &round).unwrap();
        assert!(won, "Both correct - should win");
        assert_eq!(base, 1_700_000_000, "base={}", base);
        // final = 1.7e9 × 1.05e9 / 1e9 = 1.785e9
        assert_eq!(final_p, 1_785_000_000, "final={}", final_p);
    }

    #[test]
    fn test_parlay_2_match_first_wrong_loses() {
        let odds = default_odds(1_500_000_000, 2_000_000_000, 1_800_000_000);
        let mut results = default_results();
        results[0] = MatchOutcome::Draw;   // prediction was HomeWin ← WRONG
        results[1] = MatchOutcome::AwayWin; // correct

        let round = make_round(results, odds);
        let bet = make_bet(&[
            make_prediction(0, 1, 600_000_000), // wrong
            make_prediction(1, 2, 400_000_000), // correct
        ], 1_050_000_000);

        let (won, _, _) = calculate_bet_payout(&bet, &round).unwrap();
        assert!(!won, "One wrong leg → entire parlay loses");
    }

    #[test]
    fn test_parlay_2_match_second_wrong_loses() {
        let odds = default_odds(1_500_000_000, 2_000_000_000, 1_800_000_000);
        let mut results = default_results();
        results[0] = MatchOutcome::HomeWin; // correct
        results[1] = MatchOutcome::Draw;     // prediction was AwayWin ← WRONG

        let round = make_round(results, odds);
        let bet = make_bet(&[
            make_prediction(0, 1, 600_000_000), // correct
            make_prediction(1, 2, 400_000_000), // wrong
        ], 1_050_000_000);

        let (won, _, _) = calculate_bet_payout(&bet, &round).unwrap();
        assert!(!won, "Second wrong leg → entire parlay loses");
    }

    #[test]
    fn test_parlay_10_match_all_correct() {
        // All 10 matches, each 100 tokens, 1.5x home odds
        // Base = 10 × 100 × 1.5 = 1500 tokens (in raw units: 1.5e12)
        // Parlay mult = 1.25x → final = 1500 × 1.25 = 1875
        let odds = default_odds(1_500_000_000, 2_000_000_000, 1_800_000_000);
        let results = [MatchOutcome::HomeWin; 10];
        let round = make_round(results, odds);

        let preds: Vec<_> = (0..10)
            .map(|i| make_prediction(i as u8, 1, 100_000_000))
            .collect();

        let bet = make_bet(&preds, 1_250_000_000); // 1.25x for 10-match parlay

        let (won, base, final_p) = calculate_bet_payout(&bet, &round).unwrap();
        assert!(won);
        assert_eq!(base, 1_500_000_000); // 10 × 100m × 1.5
        assert_eq!(final_p, 1_875_000_000); // base × 1.25
    }

    #[test]
    fn test_parlay_10_match_one_wrong_loses() {
        let odds = default_odds(1_500_000_000, 2_000_000_000, 1_800_000_000);
        let mut results = [MatchOutcome::HomeWin; 10];
        results[5] = MatchOutcome::AwayWin; // mismatch at match 5

        let round = make_round(results, odds);
        let preds: Vec<_> = (0..10)
            .map(|i| make_prediction(i as u8, 1, 100_000_000))
            .collect();
        let bet = make_bet(&preds, 1_250_000_000);

        let (won, _, _) = calculate_bet_payout(&bet, &round).unwrap();
        assert!(!won, "One wrong in 10-leg parlay → full loss");
    }

    // ── odds range validation ─────────────────────────────────────────────────

    #[test]
    fn test_payout_with_min_odds_1_2x() {
        // 1.2x odds (near floor) - smallest valid payout
        let odds = default_odds(1_200_000_000, 1_200_000_000, 1_200_000_000);
        let mut results = default_results();
        results[0] = MatchOutcome::HomeWin;

        let round = make_round(results, odds);
        let bet = make_bet(&[make_prediction(0, 1, 1_000_000_000)], SCALE);

        let (won, _, final_p) = calculate_bet_payout(&bet, &round).unwrap();
        assert!(won);
        assert_eq!(final_p, 1_200_000_000); // exactly 1.2x
    }

    #[test]
    fn test_payout_with_max_odds_2_2x() {
        // 2.2x odds (ceiling) - largest valid single payout
        let odds = default_odds(2_200_000_000, 2_200_000_000, 2_200_000_000);
        let mut results = default_results();
        results[0] = MatchOutcome::AwayWin;

        let round = make_round(results, odds);
        let bet = make_bet(&[make_prediction(0, 2, 1_000_000_000)], SCALE);

        let (won, _, final_p) = calculate_bet_payout(&bet, &round).unwrap();
        assert!(won);
        assert_eq!(final_p, 2_200_000_000); // exactly 2.2x
    }

    // ── team token odds boost ─────────────────────────────────────────────────

    #[test]
    fn test_team_token_boost_increases_multiplier() {
        use crate::constants::*;
        // Base parlay multiplier: 1.0x (single bet)
        let base = PARLAY_MULTIPLIER_1_MATCH; // 1.0 × SCALE
        // 5% boost
        let boost = (base as u128 * TEAM_TOKEN_ODDS_BOOST_BPS as u128 / BPS_DENOMINATOR as u128) as u64;
        let boosted = base + boost;

        // boosted should be 1.05x
        assert_eq!(boosted, 1_050_000_000, "5% boost on 1.0x = 1.05x");

        let odds = default_odds(1_500_000_000, 2_000_000_000, 1_800_000_000);
        let mut results = default_results();
        results[0] = MatchOutcome::HomeWin;
        let round = make_round(results, odds);

        // Without boost
        let bet_no_boost = make_bet(&[make_prediction(0, 1, 1_000_000_000)], base);
        let (_, _, p_no_boost) = calculate_bet_payout(&bet_no_boost, &round).unwrap();

        // With boost
        let bet_boosted = make_bet(&[make_prediction(0, 1, 1_000_000_000)], boosted);
        let (_, _, p_boosted) = calculate_bet_payout(&bet_boosted, &round).unwrap();

        assert!(p_boosted > p_no_boost, "Boosted payout ({}) should exceed normal ({})", p_boosted, p_no_boost);
    }

    // ── max payout cap ────────────────────────────────────────────────────────

    #[test]
    fn test_max_payout_cap_applied() {
        // 10-leg parlay with MAX_BET_AMOUNT per leg at 2.2x odds, 1.25x parlay mult:
        // raw = 10 × MAX_BET_AMOUNT × 2.2 × 1.25 = 275_000_000_000_000 > MAX_PAYOUT_PER_BET
        let odds = default_odds(2_200_000_000, 2_200_000_000, 2_200_000_000);
        let results = [MatchOutcome::HomeWin; 10];
        let round = make_round(results, odds);

        let preds: Vec<_> = (0..10)
            .map(|i| make_prediction(i as u8, 1, MAX_BET_AMOUNT))
            .collect();
        let bet = make_bet(&preds, 1_250_000_000); // 1.25x for 10-match parlay

        let (won, _, final_p) = calculate_bet_payout(&bet, &round).unwrap();
        assert!(won);
        // raw payout = 10 × 10_000 tokens × 2.2 × 1.25 = 275_000 tokens
        // MAX_PAYOUT_PER_BET = 100_000 tokens — cap should be applied
        assert_eq!(final_p, MAX_PAYOUT_PER_BET, "Payout must be capped at MAX_PAYOUT_PER_BET");
    }
}
