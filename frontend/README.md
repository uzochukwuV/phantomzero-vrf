# PhantomZero Sportsbook Frontend

A modern, decentralized sportsbook frontend built with Next.js 14 and Solana wallet adapters.

## Features

- 🎲 **VRF-Powered Betting**: Provably fair outcomes using Switchboard VRF
- 💰 **Locked Odds**: Odds fixed at seeding time - never change after you bet
- 🚀 **Parlay Bonuses**: Multi-match bets get bonus multipliers up to 1.25x
- 💧 **Liquidity Pool**: Provide liquidity to earn fees from betting activity
- 🔗 **Wallet Integration**: Support for Phantom, Solflare, and other Solana wallets
- 📊 **Real-time Round Info**: Track betting rounds, match results, and your bets

## Tech Stack

- **Framework**: Next.js 14 (App Router)
- **Language**: TypeScript
- **Styling**: Tailwind CSS
- **Blockchain**: Solana (via @coral-xyz/anchor & @solana/web3.js)
- **Wallet Adapters**: @solana/wallet-adapter-react
- **UI Components**: Custom React components

## Getting Started

### Prerequisites

- Node.js 18+ and npm
- A Solana wallet (Phantom, Solflare, etc.)
- Some devnet SOL for testing

### Installation

```bash
# Install dependencies
npm install

# Run development server
npm run dev

# Build for production
npm run build

# Start production server
npm start
```

Visit `http://localhost:3000` to see the app.

## Project Structure

```
frontend/
├── app/
│   ├── layout.tsx       # Root layout with wallet provider
│   ├── page.tsx         # Main page with tabs
│   └── globals.css      # Global styles
├── components/
│   ├── BettingInterface.tsx      # Place bets UI
│   ├── LiquidityPool.tsx         # LP management UI
│   ├── RoundInfo.tsx             # Round and results UI
│   └── WalletContextProvider.tsx # Wallet connection provider
├── utils/
│   └── anchor.ts        # Anchor program & PDA helpers
└── public/              # Static assets
```

## Main Components

### BettingInterface

The main betting interface where users can:
- Select matches and outcomes (Home/Draw/Away)
- Place single bets or parlays
- See expected payouts with parlay bonuses
- View locked odds (1.25x - 1.95x range)

### LiquidityPool

Liquidity provider interface:
- Deposit tokens to earn fees
- Withdraw liquidity with shares
- Track pool performance and APY
- Monitor total liquidity and reserves

### RoundInfo

Round information dashboard:
- View current and past rounds
- See match results and pool distribution
- Track your active bets
- Claim winnings from settled rounds
- View VRF randomness proofs

## Configuration

### Program ID

Update the program ID in `utils/anchor.ts`:

```typescript
export const PROGRAM_ID = new PublicKey('YOUR_PROGRAM_ID_HERE');
```

### Network

Change the network in `components/WalletContextProvider.tsx`:

```typescript
const network = WalletAdapterNetwork.Devnet; // or Mainnet
```

### Custom RPC

Provide a custom RPC endpoint for better performance:

```typescript
const endpoint = 'https://your-custom-rpc.com';
```

## Integration with Smart Contract

The frontend is designed to work with the Anchor smart contract located at `../smart-contract/`.

### Required Steps:

1. **Generate IDL**: After building the smart contract, copy the IDL file:
   ```bash
   cp ../smart-contract/target/idl/sportsbook.json ./utils/
   ```

2. **Update anchor.ts**: Import the generated IDL:
   ```typescript
   import SPORTSBOOK_IDL from './sportsbook.json';
   ```

3. **Implement Transactions**: Complete the TODO sections in:
   - `BettingInterface.tsx` - Implement `handlePlaceBet()`
   - `LiquidityPool.tsx` - Implement `handleDeposit()` and `handleWithdraw()`
   - `RoundInfo.tsx` - Implement claim winnings functionality

## Features Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Wallet Connection | ✅ Complete | Phantom, Solflare, WalletConnect |
| UI Components | ✅ Complete | All 3 main interfaces |
| Betting Interface | ⚠️ Partial | UI done, transactions TODO |
| Liquidity Pool | ⚠️ Partial | UI done, transactions TODO |
| Round Info | ⚠️ Partial | UI done, data fetching TODO |
| Place Bet Transaction | ❌ TODO | Need to integrate with contract |
| Add/Remove Liquidity | ❌ TODO | Need to integrate with contract |
| Claim Winnings | ❌ TODO | Need to integrate with contract |
| Fetch On-Chain Data | ❌ TODO | Need to subscribe to accounts |

## Development Notes

### Mock Data

Currently the app uses mock data for demonstration. To integrate with the actual contract:

1. Fetch betting pool and liquidity pool accounts
2. Subscribe to round accounting accounts
3. Fetch user's bets and LP positions
4. Implement transaction signing and sending

### Error Handling

Add proper error handling for:
- Transaction failures
- Insufficient balance
- Network errors
- Invalid bet selections

### State Management

Consider adding state management (like Zustand or Jotai) if the app grows in complexity.

## Deployment

### Vercel (Recommended)

```bash
# Install Vercel CLI
npm install -g vercel

# Deploy
vercel
```

### Other Platforms

The app can be deployed to any platform that supports Next.js:
- Netlify
- AWS Amplify
- Docker containers

## Security Considerations

- ⚠️ **This is a demo application** - Additional security measures needed for production
- Always validate transaction parameters client-side
- Use environment variables for sensitive configuration
- Implement rate limiting for API calls
- Add CAPTCHA for transaction-heavy operations

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## License

MIT

## Support

For issues and questions:
- GitHub Issues: Create an issue in the repository
- Smart Contract Docs: See `../smart-contract/README.md`

---

Built with ❤️ on Solana
