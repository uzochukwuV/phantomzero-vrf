'use client';

import { useState } from 'react';
import { useWallet, useConnection } from '@solana/wallet-adapter-react';

export default function LiquidityPool() {
  const { publicKey } = useWallet();
  const { connection } = useConnection();

  const [depositAmount, setDepositAmount] = useState<string>('');
  const [withdrawShares, setWithdrawShares] = useState<string>('');
  const [loading, setLoading] = useState(false);

  // Mock LP data - in production, fetch from chain
  const lpData = {
    totalLiquidity: 1000000,
    totalShares: 950000,
    yourShares: 5000,
    yourLiquidity: 5263.16,
    availableLiquidity: 850000,
    lockedReserve: 150000,
    totalProfit: 75000,
    totalLoss: 25000,
    apy: 12.5,
  };

  const handleDeposit = async () => {
    if (!publicKey || !depositAmount) return;

    setLoading(true);
    try {
      // TODO: Implement actual transaction
      // 1. Get program and liquidity pool
      // 2. Transfer tokens to pool
      // 3. Call add_liquidity instruction
      // 4. Receive LP shares

      console.log('Depositing:', depositAmount);
      alert('Deposit functionality coming soon!');
    } catch (error) {
      console.error('Error depositing:', error);
      alert('Failed to deposit: ' + error);
    } finally {
      setLoading(false);
    }
  };

  const handleWithdraw = async () => {
    if (!publicKey || !withdrawShares) return;

    setLoading(true);
    try {
      // TODO: Implement actual transaction
      // 1. Get program and liquidity pool
      // 2. Call remove_liquidity instruction
      // 3. Receive tokens back

      console.log('Withdrawing shares:', withdrawShares);
      alert('Withdraw functionality coming soon!');
    } catch (error) {
      console.error('Error withdrawing:', error);
      alert('Failed to withdraw: ' + error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold mb-4">Liquidity Pool</h2>
        <p className="text-gray-400">
          Provide liquidity to earn fees from betting activity. Your liquidity helps fund
          payouts and seed new rounds.
        </p>
      </div>

      {/* Pool Stats */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-gray-700 p-4 rounded-lg">
          <div className="text-sm text-gray-400 mb-1">Total Liquidity</div>
          <div className="text-2xl font-bold text-primary">
            {lpData.totalLiquidity.toLocaleString()}
          </div>
        </div>

        <div className="bg-gray-700 p-4 rounded-lg">
          <div className="text-sm text-gray-400 mb-1">Available</div>
          <div className="text-2xl font-bold text-green-400">
            {lpData.availableLiquidity.toLocaleString()}
          </div>
        </div>

        <div className="bg-gray-700 p-4 rounded-lg">
          <div className="text-sm text-gray-400 mb-1">Locked Reserve</div>
          <div className="text-2xl font-bold text-yellow-400">
            {lpData.lockedReserve.toLocaleString()}
          </div>
        </div>

        <div className="bg-gray-700 p-4 rounded-lg">
          <div className="text-sm text-gray-400 mb-1">Est. APY</div>
          <div className="text-2xl font-bold text-primary">
            {lpData.apy.toFixed(1)}%
          </div>
        </div>
      </div>

      {/* Your Position */}
      <div className="bg-gray-700 p-6 rounded-lg">
        <h3 className="text-xl font-bold mb-4">Your Position</h3>

        {lpData.yourShares > 0 ? (
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <div className="text-sm text-gray-400 mb-1">Your Shares</div>
              <div className="text-xl font-bold">{lpData.yourShares.toLocaleString()}</div>
            </div>

            <div>
              <div className="text-sm text-gray-400 mb-1">Your Liquidity</div>
              <div className="text-xl font-bold text-primary">
                {lpData.yourLiquidity.toFixed(2)}
              </div>
            </div>

            <div>
              <div className="text-sm text-gray-400 mb-1">Pool Share</div>
              <div className="text-xl font-bold">
                {((lpData.yourShares / lpData.totalShares) * 100).toFixed(2)}%
              </div>
            </div>

            <div>
              <div className="text-sm text-gray-400 mb-1">Earnings</div>
              <div className="text-xl font-bold text-green-400">
                +{((lpData.yourLiquidity - (lpData.yourShares * lpData.totalLiquidity / lpData.totalShares)) / lpData.yourLiquidity * 100).toFixed(2)}%
              </div>
            </div>
          </div>
        ) : (
          <p className="text-gray-400">
            You don't have any liquidity in the pool yet. Deposit tokens to start earning.
          </p>
        )}
      </div>

      {/* Actions */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Deposit */}
        <div className="bg-gray-700 p-6 rounded-lg">
          <h3 className="text-xl font-bold mb-4">Deposit Liquidity</h3>

          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium mb-2">Amount (tokens)</label>
              <input
                type="number"
                value={depositAmount}
                onChange={(e) => setDepositAmount(e.target.value)}
                placeholder="0.00"
                className="w-full px-4 py-2 bg-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
                min="0"
                step="0.01"
              />
            </div>

            {depositAmount && (
              <div className="bg-gray-600 p-4 rounded-lg">
                <div className="flex justify-between text-sm mb-2">
                  <span className="text-gray-400">You will receive:</span>
                  <span className="font-bold">
                    ~{Math.floor(parseFloat(depositAmount) * (lpData.totalShares / lpData.totalLiquidity)).toLocaleString()} shares
                  </span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-gray-400">Pool share:</span>
                  <span className="font-bold">
                    {((parseFloat(depositAmount) / (lpData.totalLiquidity + parseFloat(depositAmount))) * 100).toFixed(2)}%
                  </span>
                </div>
              </div>
            )}

            <button
              onClick={handleDeposit}
              disabled={loading || !depositAmount}
              className="w-full bg-primary text-dark font-bold py-3 rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {loading ? 'Depositing...' : 'Deposit'}
            </button>
          </div>
        </div>

        {/* Withdraw */}
        <div className="bg-gray-700 p-6 rounded-lg">
          <h3 className="text-xl font-bold mb-4">Withdraw Liquidity</h3>

          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium mb-2">Shares to withdraw</label>
              <input
                type="number"
                value={withdrawShares}
                onChange={(e) => setWithdrawShares(e.target.value)}
                placeholder="0"
                className="w-full px-4 py-2 bg-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
                min="0"
                max={lpData.yourShares}
                step="1"
              />
              <div className="text-xs text-gray-400 mt-1">
                Available: {lpData.yourShares.toLocaleString()} shares
              </div>
            </div>

            {withdrawShares && (
              <div className="bg-gray-600 p-4 rounded-lg">
                <div className="flex justify-between text-sm">
                  <span className="text-gray-400">You will receive:</span>
                  <span className="font-bold">
                    ~{(parseFloat(withdrawShares) * lpData.totalLiquidity / lpData.totalShares).toFixed(2)} tokens
                  </span>
                </div>
              </div>
            )}

            <button
              onClick={handleWithdraw}
              disabled={loading || !withdrawShares || parseFloat(withdrawShares) > lpData.yourShares}
              className="w-full bg-red-500 text-white font-bold py-3 rounded-lg hover:bg-red-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {loading ? 'Withdrawing...' : 'Withdraw'}
            </button>
          </div>
        </div>
      </div>

      {/* Performance Stats */}
      <div className="bg-gray-700 p-6 rounded-lg">
        <h3 className="text-xl font-bold mb-4">Pool Performance</h3>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="bg-gray-600 p-4 rounded-lg">
            <div className="text-sm text-gray-400 mb-1">Total Profit</div>
            <div className="text-xl font-bold text-green-400">
              +{lpData.totalProfit.toLocaleString()}
            </div>
          </div>

          <div className="bg-gray-600 p-4 rounded-lg">
            <div className="text-sm text-gray-400 mb-1">Total Loss</div>
            <div className="text-xl font-bold text-red-400">
              -{lpData.totalLoss.toLocaleString()}
            </div>
          </div>

          <div className="bg-gray-600 p-4 rounded-lg">
            <div className="text-sm text-gray-400 mb-1">Net Performance</div>
            <div className="text-xl font-bold text-primary">
              +{(lpData.totalProfit - lpData.totalLoss).toLocaleString()}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
