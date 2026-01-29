# VRF Integration for Solana Sportsbook

## Overview

This document outlines how to integrate Verifiable Random Functions (VRF) into the sportsbook contract for provably fair match outcome generation.

## VRF Providers on Solana

### 1. Switchboard V2 (Recommended)

Switchboard is the most mature VRF solution on Solana.

```rust
use switchboard_v2::VrfAccountData;

// In your settle_round instruction, verify VRF proof
pub fn settle_round_with_vrf(
    ctx: Context<SettleRound>,
    round_id: u64,
) -> Result<()> {
    // Load VRF account data
    let vrf = VrfAccountData::new(ctx.accounts.vrf_account)?;

    // Verify VRF proof
    let result_buffer = vrf.get_result()?;

    // Extract randomness for each match (10 matches)
    let mut match_results = Vec::with_capacity(10);
    for i in 0..10 {
        // Use 32 bytes of randomness per match
        let offset = i * 32;
        let match_randomness = &result_buffer[offset..offset+32];

        // Convert to match outcome (1, 2, or 3)
        let outcome = calculate_outcome_from_randomness(match_randomness);
        match_results.push(outcome);
    }

    // Continue with settlement using VRF-generated outcomes
    // ... rest of settlement logic
}

fn calculate_outcome_from_randomness(randomness: &[u8]) -> u8 {
    // Convert randomness to u64
    let value = u64::from_le_bytes(randomness[0..8].try_into().unwrap());

    // Map to outcome (1, 2, or 3) with equal probability
    // This gives 33.33% chance for each outcome
    ((value % 3) + 1) as u8
}
```

### 2. Chainlink VRF

Chainlink also provides VRF on Solana (in beta).

```rust
// Example Chainlink VRF integration
pub fn request_randomness(
    ctx: Context<RequestRandomness>,
    round_id: u64,
) -> Result<()> {
    // Request VRF from Chainlink
    // This would trigger an off-chain oracle
    // Results delivered via callback
}
```

## Implementation Steps

### Step 1: Add VRF Account to SettleRound

```rust
#[derive(Accounts)]
pub struct SettleRound<'info> {
    // ... existing accounts

    /// Switchboard VRF account
    /// CHECK: Validated by Switchboard program
    pub vrf_account: UncheckedAccount<'info>,

    /// Switchboard program
    /// CHECK: This is the Switchboard VRF program ID
    pub switchboard_program: UncheckedAccount<'info>,
}
```

### Step 2: Request VRF Before Settlement

```rust
pub fn request_round_randomness(
    ctx: Context<RequestRandomness>,
    round_id: u64,
) -> Result<()> {
    // Initialize VRF request
    // This typically involves:
    // 1. Creating a VRF account
    // 2. Funding it with SOL for oracle fees
    // 3. Requesting randomness from the oracle network

    // The VRF result will be available after oracle fulfillment
    // Then call settle_round_with_vrf
}
```

### Step 3: Integrate with Game Engine

For a complete implementation, you'd integrate with a separate game engine contract:

```rust
// Cross-program invocation to game engine
pub fn settle_round(
    ctx: Context<SettleRound>,
    round_id: u64,
) -> Result<()> {
    // Call game engine to generate match results using VRF
    let cpi_accounts = GenerateResults {
        game_engine: ctx.accounts.game_engine.to_account_info(),
        vrf_account: ctx.accounts.vrf_account.to_account_info(),
        // ... other accounts
    };

    let cpi_ctx = CpiContext::new(
        ctx.accounts.game_engine_program.to_account_info(),
        cpi_accounts,
    );

    game_engine::cpi::generate_match_results(cpi_ctx, round_id)?;

    // Results are stored in game engine
    // Fetch and use for settlement
}
```

## Security Considerations

1. **VRF Proof Verification**: Always verify VRF proofs to ensure randomness is genuine
2. **Request-Fulfill Pattern**: Use two-step process (request, then fulfill)
3. **Oracle Fees**: Ensure VRF accounts are properly funded
4. **Replay Protection**: Prevent reuse of old VRF results

## Cost Estimation

- Switchboard VRF request: ~0.002 SOL per request
- Chainlink VRF: Variable based on network conditions
- Recommend: Protocol covers VRF costs from fees

## Testing VRF Integration

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vrf_settlement() {
        // Mock VRF result
        let mock_randomness = [0u8; 320]; // 32 bytes per match * 10

        // Test outcome generation
        for i in 0..10 {
            let outcome = calculate_outcome_from_randomness(
                &mock_randomness[i*32..(i+1)*32]
            );
            assert!(outcome >= 1 && outcome <= 3);
        }
    }
}
```

## Production Deployment

1. Deploy on devnet first with Switchboard devnet oracles
2. Test full betting cycle with VRF
3. Migrate to mainnet with production oracle network
4. Monitor VRF fulfillment times and costs

## Alternative: Chainlink Automation

For automated round settlement:

```rust
// Chainlink Keepers can trigger settlement automatically
// when VRF results are available
pub fn check_upkeep() -> bool {
    // Return true if round is ready to settle
    // (VRF fulfilled, betting closed, etc.)
}

pub fn perform_upkeep() {
    // Called by Chainlink Keepers
    // Settles round automatically
}
```
