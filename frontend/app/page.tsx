'use client';

import { useWallet } from '@solana/wallet-adapter-react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { useState } from 'react';
import BettingInterface from '@/components/BettingInterface';
import LiquidityPool from '@/components/LiquidityPool';
import RoundInfo from '@/components/RoundInfo';

export default function Home() {
  const { connected } = useWallet();
  const [activeTab, setActiveTab] = useState<'bet' | 'pool' | 'info'>('bet');

  return (
    <main className="min-h-screen p-8">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <header className="flex justify-between items-center mb-8">
          <div>
            <h1 className="text-4xl font-bold text-primary mb-2">
              PhantomZero Sportsbook
            </h1>
            <p className="text-gray-400">
              Decentralized betting powered by Solana & VRF randomness
            </p>
          </div>
          <WalletMultiButton />
        </header>

        {!connected ? (
          <div className="bg-gray-800 rounded-lg p-12 text-center">
            <h2 className="text-2xl font-bold mb-4">Welcome to PhantomZero Sportsbook</h2>
            <p className="text-gray-400 mb-6">
              Connect your wallet to start betting on matches
            </p>
            <WalletMultiButton />

            <div className="mt-12 grid grid-cols-1 md:grid-cols-3 gap-6 text-left">
              <div className="bg-gray-700 p-6 rounded-lg">
                <h3 className="text-xl font-bold mb-2 text-primary">🎲 VRF Randomness</h3>
                <p className="text-sm text-gray-300">
                  Provably fair match outcomes powered by Switchboard VRF
                </p>
              </div>
              <div className="bg-gray-700 p-6 rounded-lg">
                <h3 className="text-xl font-bold mb-2 text-primary">💰 Locked Odds</h3>
                <p className="text-sm text-gray-300">
                  Odds locked at seeding time - never change after you bet
                </p>
              </div>
              <div className="bg-gray-700 p-6 rounded-lg">
                <h3 className="text-xl font-bold mb-2 text-primary">🚀 Parlay Bonuses</h3>
                <p className="text-sm text-gray-300">
                  Multi-match bets get bonus multipliers up to 1.25x
                </p>
              </div>
            </div>
          </div>
        ) : (
          <>
            {/* Tab Navigation */}
            <div className="flex gap-4 mb-6">
              <button
                onClick={() => setActiveTab('bet')}
                className={`px-6 py-3 rounded-lg font-semibold transition-colors ${
                  activeTab === 'bet'
                    ? 'bg-primary text-dark'
                    : 'bg-gray-800 text-gray-300 hover:bg-gray-700'
                }`}
              >
                Place Bets
              </button>
              <button
                onClick={() => setActiveTab('pool')}
                className={`px-6 py-3 rounded-lg font-semibold transition-colors ${
                  activeTab === 'pool'
                    ? 'bg-primary text-dark'
                    : 'bg-gray-800 text-gray-300 hover:bg-gray-700'
                }`}
              >
                Liquidity Pool
              </button>
              <button
                onClick={() => setActiveTab('info')}
                className={`px-6 py-3 rounded-lg font-semibold transition-colors ${
                  activeTab === 'info'
                    ? 'bg-primary text-dark'
                    : 'bg-gray-800 text-gray-300 hover:bg-gray-700'
                }`}
              >
                Round Info
              </button>
            </div>

            {/* Content */}
            <div className="bg-gray-800 rounded-lg p-6">
              {activeTab === 'bet' && <BettingInterface />}
              {activeTab === 'pool' && <LiquidityPool />}
              {activeTab === 'info' && <RoundInfo />}
            </div>
          </>
        )}

        {/* Footer */}
        <footer className="mt-12 text-center text-gray-500 text-sm">
          <p>Built on Solana • Powered by Anchor Framework • VRF by Switchboard</p>
          <p className="mt-2">⚠️ This is a demo application. Bet responsibly.</p>
        </footer>
      </div>
    </main>
  );
}
