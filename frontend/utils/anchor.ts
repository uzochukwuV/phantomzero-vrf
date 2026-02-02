import { Program, AnchorProvider, Idl, setProvider } from '@coral-xyz/anchor';
import { Connection, PublicKey } from '@solana/web3.js';
import { AnchorWallet } from '@solana/wallet-adapter-react';

// Replace with your actual program ID
export const PROGRAM_ID = new PublicKey('Spo7t11111111111111111111111111111111111111');

// You'll need to copy the IDL from your Anchor program
// This is a placeholder - you need to generate the actual IDL with `anchor build`
export const SPORTSBOOK_IDL: Idl = {
  version: '0.1.0',
  name: 'sportsbook',
  instructions: [],
  accounts: [],
};

export function getProgram(connection: Connection, wallet: AnchorWallet) {
  const provider = new AnchorProvider(connection, wallet, {
    commitment: 'confirmed',
  });
  setProvider(provider);

  const program = new Program(SPORTSBOOK_IDL, PROGRAM_ID, provider);
  return program;
}

// PDA helpers
export function getBettingPoolPDA(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('betting_pool')],
    PROGRAM_ID
  );
}

export function getLiquidityPoolPDA(bettingPool: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('liquidity_pool'), bettingPool.toBuffer()],
    PROGRAM_ID
  );
}

export function getRoundAccountingPDA(
  bettingPool: PublicKey,
  roundId: number
): [PublicKey, number] {
  const roundIdBuffer = Buffer.alloc(8);
  roundIdBuffer.writeBigUInt64LE(BigInt(roundId));

  return PublicKey.findProgramAddressSync(
    [Buffer.from('round'), bettingPool.toBuffer(), roundIdBuffer],
    PROGRAM_ID
  );
}

export function getBetPDA(
  bettingPool: PublicKey,
  betId: number
): [PublicKey, number] {
  const betIdBuffer = Buffer.alloc(8);
  betIdBuffer.writeBigUInt64LE(BigInt(betId));

  return PublicKey.findProgramAddressSync(
    [Buffer.from('bet'), bettingPool.toBuffer(), betIdBuffer],
    PROGRAM_ID
  );
}
