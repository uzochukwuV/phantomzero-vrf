# QuickStart Guide - PhantomZero VRF Sportsbook

Get your Solana sportsbook contract up and running in minutes!

## 🚀 Quick Setup

### 1. Install Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Install Anchor CLI
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install 0.29.0
avm use 0.29.0

# Verify installations
solana --version
anchor --version
```

### 2. Configure Solana

```bash
# Set cluster to devnet
solana config set --url devnet

# Create a wallet (if you don't have one)
solana-keygen new --outfile ~/.config/solana/id.json

# Get some devnet SOL
solana airdrop 2

# Verify balance
solana balance
```

### 3. Build the Program

```bash
# Navigate to smart-contract directory
cd smart-contract

# Install Node dependencies (for tests)
npm install

# Build the program
anchor build

# This generates:
# - target/deploy/sportsbook.so (compiled program)
# - target/idl/sportsbook.json (Interface Definition Language)
# - target/types/sportsbook.ts (TypeScript types)
```

### 4. Deploy to Devnet

```bash
# Deploy
anchor deploy --provider.cluster devnet

# You'll see output like:
# Program Id: Sports11111111111111111111111111111111111
```

### 5. Update Program ID

```bash
# Copy the Program ID from deployment output

# Update Anchor.toml (all three sections)
[programs.localnet]
sportsbook = "YOUR_PROGRAM_ID"

[programs.devnet]
sportsbook = "YOUR_PROGRAM_ID"

[programs.mainnet]
sportsbook = "YOUR_PROGRAM_ID"

# Update lib.rs
declare_id!("YOUR_PROGRAM_ID");

# Rebuild
anchor build

# Redeploy
anchor deploy --provider.cluster devnet
```

## 🎮 Usage Examples

### Initialize the Betting Pool

```typescript
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Sportsbook } from "../target/types/sportsbook";

const program = anchor.workspace.Sportsbook as Program<Sportsbook>;

// Derive betting pool PDA
const [bettingPoolPda] = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("betting_pool")],
  program.programId
);

// Initialize
await program.methods
  .initialize(
    500,  // 5% protocol fee
    2500, // 25% winner share
    200   // 2% season pool share
  )
  .accounts({
    bettingPool: bettingPoolPda,
    authority: provider.wallet.publicKey,
    tokenMint: leagueTokenMint,
    protocolTreasury: treasuryAddress,
    systemProgram: anchor.web3.SystemProgram.programId,
  })
  .rpc();

console.log("Betting pool initialized!");
```

### Create a New Round

```typescript
const roundId = new anchor.BN(1);

const [roundPda] = anchor.web3.PublicKey.findProgramAddressSync(
  [
    Buffer.from("round"),
    bettingPoolPda.toBuffer(),
    roundId.toArrayLike(Buffer, "le", 8),
  ],
  program.programId
);

await program.methods
  .initializeRound(roundId)
  .accounts({
    bettingPool: bettingPoolPda,
    roundAccounting: roundPda,
    authority: provider.wallet.publicKey,
    systemProgram: anchor.web3.SystemProgram.programId,
  })
  .rpc();

console.log("Round initialized!");
```

### Seed Round Pools

```typescript
await program.methods
  .seedRoundPools(roundId)
  .accounts({
    bettingPool: bettingPoolPda,
    roundAccounting: roundPda,
    liquidityPool: liquidityPoolPda,
    lpTokenAccount: lpTokenAccountAddress,
    bettingPoolTokenAccount: bettingPoolTokenAccountAddress,
    authority: provider.wallet.publicKey,
    tokenProgram: TOKEN_PROGRAM_ID,
  })
  .rpc();

console.log("Round seeded! Odds locked.");
```

### Place a Bet

```typescript
const betId = new anchor.BN(1);

const [betPda] = anchor.web3.PublicKey.findProgramAddressSync(
  [
    Buffer.from("bet"),
    bettingPoolPda.toBuffer(),
    betId.toArrayLike(Buffer, "le", 8),
  ],
  program.programId
);

await program.methods
  .placeBet(
    roundId,
    [0, 1, 2],              // Match indices
    [1, 2, 3],              // Outcomes (Home, Away, Draw)
    new anchor.BN(1000_000_000_000) // 1000 tokens (9 decimals)
  )
  .accounts({
    bettingPool: bettingPoolPda,
    roundAccounting: roundPda,
    liquidityPool: liquidityPoolPda,
    bet: betPda,
    bettorTokenAccount: userTokenAccount,
    bettingPoolTokenAccount: bettingPoolTokenAccountAddress,
    protocolTreasuryTokenAccount: treasuryTokenAccount,
    lpTokenAccount: lpTokenAccountAddress,
    bettor: provider.wallet.publicKey,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: anchor.web3.SystemProgram.programId,
  })
  .rpc();

console.log("Bet placed!");
```

### Settle Round

```typescript
const matchResults = [1, 2, 1, 3, 2, 1, 3, 2, 1, 2]; // Results for 10 matches

await program.methods
  .settleRound(roundId, matchResults)
  .accounts({
    bettingPool: bettingPoolPda,
    roundAccounting: roundPda,
    authority: provider.wallet.publicKey,
  })
  .rpc();

console.log("Round settled!");
```

### Claim Winnings

```typescript
await program.methods
  .claimWinnings(
    betId,
    new anchor.BN(900_000_000_000) // Minimum acceptable payout
  )
  .accounts({
    bettingPool: bettingPoolPda,
    roundAccounting: roundPda,
    bet: betPda,
    liquidityPool: liquidityPoolPda,
    bettingPoolTokenAccount: bettingPoolTokenAccountAddress,
    lpTokenAccount: lpTokenAccountAddress,
    bettorTokenAccount: userTokenAccount,
    bettor: provider.wallet.publicKey,
    tokenProgram: TOKEN_PROGRAM_ID,
  })
  .rpc();

console.log("Winnings claimed!");
```

## 🧪 Running Tests

```bash
# Run all tests
anchor test

# Run with verbose output
anchor test -- --nocapture

# Run specific test
anchor test -- test_place_bet
```

## 📊 Monitoring

### Check Program Logs

```bash
# View recent logs
solana logs --url devnet | grep "Program Sports"

# Follow logs in real-time
solana logs --url devnet --program-id Sports11111111111111111111111111111111111
```

### Inspect Accounts

```bash
# View betting pool account
solana account <BETTING_POOL_PDA> --url devnet --output json-compact

# View round accounting
solana account <ROUND_PDA> --url devnet --output json-compact
```

## 🔍 Troubleshooting

### Issue: "Insufficient funds"

```bash
# Get more devnet SOL
solana airdrop 2 --url devnet
```

### Issue: "Program account not found"

```bash
# Verify program is deployed
solana program show <PROGRAM_ID> --url devnet

# If not found, redeploy
anchor deploy --provider.cluster devnet
```

### Issue: "Invalid account data for instruction"

- Check that all PDA derivations match the program
- Verify account sizes in state definitions
- Ensure correct seeds for PDAs

### Issue: Build errors

```bash
# Clean and rebuild
anchor clean
cargo clean
anchor build
```

## 📚 Next Steps

1. **Integrate VRF**: See `vrf_integration.md` for Switchboard VRF setup
2. **Add Frontend**: Build a React app using `@coral-xyz/anchor`
3. **Deploy to Mainnet**: After thorough testing, deploy to production
4. **Monitor & Iterate**: Use Solana explorers and analytics

## 🆘 Getting Help

- **Anchor Docs**: https://www.anchor-lang.com/
- **Solana Docs**: https://docs.solana.com/
- **Discord**: Join Anchor and Solana Discord servers
- **GitHub Issues**: Report bugs in this repository

---

Happy building! 🚀
