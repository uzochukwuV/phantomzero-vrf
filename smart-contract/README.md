# PhantomZero VRF - Solana Sportsbook Contract

A sophisticated on-chain sportsbook betting system powered by VRF randomness, built on Solana using the Anchor framework.

## 🎯 Overview

PhantomZero VRF is a decentralized sportsbook betting platform that combines:
- **VRF-powered randomness** for provably fair match outcomes
- **Locked odds system** (1.25x - 1.95x range) for predictable payouts
- **Liquidity Pool (LP) model** with AMM-style shares
- **Parlay betting** with dynamic multipliers
- **Risk management** caps to protect protocol solvency

## 🏗️ Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                     Betting Pool (Global)                     │
│  - Protocol configuration                                     │
│  - Fee settings (5% default)                                 │
│  - Season reward pool                                        │
└───────────────┬─────────────────────────────────────────────┘
                │
                ├─── Liquidity Pool (LP)
                │    - Total liquidity & shares
                │    - Profit/loss tracking
                │    - AMM-style deposits/withdrawals
                │
                ├─── Round Accounting (per round)
                │    - 10 match pools per round
                │    - Locked odds (fixed at seeding)
                │    - Revenue tracking
                │    - Parlay count (FOMO mechanism)
                │
                └─── Bets (user bets)
                     - Predictions (1-10 matches)
                     - Locked multiplier
                     - Odds-weighted allocations
```

### Key Features

#### 1. Locked Odds System

Odds are **locked at round seeding** and never change, ensuring:
- ✅ Predictable payouts for users
- ✅ Exact accounting (no AMM slippage)
- ✅ Fair odds in 1.25x - 1.95x range

```rust
// Odds locked once at seeding
pub struct LockedOdds {
    pub home_odds: u64,  // e.g., 1.5e9 = 1.5x
    pub away_odds: u64,
    pub draw_odds: u64,
    pub locked: bool,    // Never changes after seeding
}
```

#### 2. Odds-Weighted Allocation

Parlay bets use **odds-weighted allocation** to ensure fair payouts:

```
User bets $100 on 3-match parlay:
- Match 1 (Home): Odds 1.5x → Allocate $25
- Match 2 (Away): Odds 1.8x → Allocate $20
- Match 3 (Draw): Odds 1.4x → Allocate $30

Total allocated: $75 (remaining $25 borrowed from LP)
Target payout: $100 × 1.5 × 1.8 × 1.4 × parlay_multiplier
```

This ensures each match contributes **equally** to the final payout.

#### 3. Liquidity Pool (LP) Model

The LP pool operates like an AMM:

```rust
// Add liquidity
shares = (deposit_amount × total_shares) / total_liquidity

// Remove liquidity
amount = (shares × total_liquidity) / total_shares
```

Features:
- 💰 Earns revenue from losing bets
- 📊 Covers all payouts (including parlay bonuses)
- 🔒 Risk managed with caps
- 📈 Transparent profit/loss tracking

#### 4. Parlay Multiplier System

**Linear progression** (reduced for LP safety):
- 1 match: 1.0x
- 2 matches: 1.05x
- 3 matches: 1.10x
- ...
- 10 matches: 1.25x (capped)

**Dynamic adjustments**:
- Pool imbalance gating (economic protection)
- Reserve-based decay (safety valve)
- Minimum 1.1x multiplier

#### 5. Risk Management

```rust
// Critical caps
const MAX_BET_AMOUNT: u64 = 10_000 tokens;
const MAX_PAYOUT_PER_BET: u64 = 100_000 tokens;
const MAX_ROUND_PAYOUTS: u64 = 500_000 tokens;
```

Protection mechanisms:
- ✅ Max bet size per user
- ✅ Max payout per winning bet
- ✅ Max total payouts per round
- ✅ LP liquidity checks before accepting bets

## 📋 Instructions

### Initialization

```rust
// 1. Initialize global betting pool
initialize(
    protocol_fee_bps: 500,      // 5%
    winner_share_bps: 2500,     // 25%
    season_pool_share_bps: 200, // 2%
)

// 2. Initialize a new round
initialize_round(round_id: 1)

// 3. Seed round pools (creates initial odds)
seed_round_pools(round_id: 1)
// → Locks odds for all 10 matches
```

### Betting Flow

```rust
// 4. Place a bet (single or parlay)
place_bet(
    round_id: 1,
    match_indices: [0, 1, 2],  // Matches 0, 1, 2
    outcomes: [1, 2, 3],        // Home, Away, Draw
    amount: 1000,               // 1000 tokens
)
// → Deducts 5% fee
// → Allocates to pools using odds-weighted allocation
// → Locks parlay multiplier
```

### Settlement & Claims

```rust
// 5. Settle round (after VRF generates results)
settle_round(
    round_id: 1,
    match_results: [1, 2, 1, 3, 2, 1, 3, 2, 1, 2],
)
// → Calculates total payouts owed

// 6. Claim winnings (pull pattern)
claim_winnings(
    bet_id: 123,
    min_payout: 900,  // Slippage protection
)
// → Pays from betting pool first
// → Pulls from LP if needed

// 7. Finalize revenue distribution
finalize_round_revenue(round_id: 1)
// → Returns remaining funds to LP
// → Allocates 2% to season pool
```

### Liquidity Management

```rust
// Add liquidity to LP
add_liquidity(amount: 10000)
// → Receives LP shares

// Remove liquidity from LP
remove_liquidity(shares: 500)
// → Burns shares, receives tokens
```

## 🔐 Security Features

### 1. Checks-Effects-Interactions Pattern

All state changes before external calls:

```rust
// ✅ CORRECT
round_accounting.lp_borrowed_for_bets += lp_borrowed;  // State first
token::transfer(cpi_ctx, lp_borrowed)?;                // External call after
```

### 2. Slippage Protection

```rust
// Users specify minimum acceptable payout
claim_winnings(bet_id, min_payout: 900)
// Reverts if actual payout < min_payout
```

### 3. Authority Checks

```rust
#[account(
    mut,
    constraint = authority.key() == betting_pool.authority
)]
pub authority: Signer<'info>,
```

### 4. Overflow Protection

```rust
// All arithmetic uses checked operations
let result = (a as u128)
    .checked_mul(b as u128)
    .ok_or(SportsbookError::CalculationOverflow)?
    .checked_div(c as u128)
    .ok_or(SportsbookError::CalculationOverflow)? as u64;
```

## 📊 Constants & Configuration

```rust
// Protocol fees
PROTOCOL_FEE_BPS: 500            // 5% on all bets
WINNER_SHARE_BPS: 2500           // 25% distributed to winners
SEASON_POOL_SHARE_BPS: 200       // 2% for season rewards

// Seeding (creates initial odds)
SEED_PER_MATCH: 3,000 tokens     // Total per match
SEED_PER_ROUND: 30,000 tokens    // Total per round (10 matches)

// Risk caps
MAX_BET_AMOUNT: 10,000 tokens
MAX_PAYOUT_PER_BET: 100,000 tokens
MAX_ROUND_PAYOUTS: 500,000 tokens

// Odds compression
MIN_COMPRESSED_ODDS: 1.25x       // Minimum odds
MAX_COMPRESSED_ODDS: 1.95x       // Maximum odds

// Parlay multipliers
MIN_PARLAY_MULTIPLIER: 1.1x
MAX_PARLAY_MULTIPLIER: 1.25x     // Capped for LP safety
```

## 🧪 Testing

```bash
# Build the program
anchor build

# Run tests
anchor test

# Deploy to devnet
anchor deploy --provider.cluster devnet
```

## 📈 Economics

### Revenue Sources (LP Pool)

1. **Losing bets**: All losing bet amounts go to LP
2. **Protocol fees**: 5% of all bets
3. **Seed recovery**: Initial seed funds returned after settlement

### Revenue Distribution

After each round:
- **LP Pool**: Gets all remaining funds minus season share
- **Season Pool**: Gets 2% of user deposits (for seasonal rewards)
- **Protocol**: Already collected 5% fee upfront

### Example Round

```
User deposits: $100,000
Protocol fee (5%): $5,000
Net betting volume: $95,000
LP seed: $30,000

Total in pool: $125,000

Winning payouts: $80,000
Remaining: $45,000

Distribution:
- Season pool (2% of $100k): $2,000
- LP profit: $43,000

LP ROI: $43,000 / $30,000 = 143% return on seed
```

## 🚀 Deployment Guide

### 1. Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Install Anchor
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install latest
avm use latest
```

### 2. Build & Deploy

```bash
# Navigate to project
cd smart-contract

# Build
anchor build

# Get program ID
solana address -k target/deploy/sportsbook-keypair.json

# Update Anchor.toml and lib.rs with program ID

# Deploy to devnet
anchor deploy --provider.cluster devnet

# Verify deployment
solana program show <PROGRAM_ID> --url devnet
```

### 3. Initialize

```bash
# Call initialize instruction
anchor run initialize-devnet
```

## 🔗 Integration

### Frontend Integration

```typescript
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Sportsbook } from "./target/types/sportsbook";

// Connect to program
const program = anchor.workspace.Sportsbook as Program<Sportsbook>;

// Place a bet
await program.methods
  .placeBet(
    new anchor.BN(1),           // round_id
    [0, 1, 2],                   // match_indices
    [1, 2, 3],                   // outcomes
    new anchor.BN(1000000000)    // amount (1000 tokens with 9 decimals)
  )
  .accounts({
    bettingPool,
    roundAccounting,
    bet,
    // ... other accounts
  })
  .rpc();
```

## 📝 License

MIT License - see LICENSE file for details

## 🤝 Contributing

Contributions welcome! Please open an issue or PR.

## 📧 Contact

- GitHub: [phantomzero-vrf](https://github.com/phantomzero-vrf)
- Twitter: [@phantomzerovrf](https://twitter.com/phantomzerovrf)

---

**⚠️ Disclaimer**: This is experimental software. Use at your own risk. Always audit smart contracts before mainnet deployment.
