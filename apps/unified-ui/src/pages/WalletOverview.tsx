import { useState, useEffect } from 'react'
import { Card, Button, Field, Input, Badge } from '../components/UI'
import { getWalletBalance, getWalletTransactions } from '../lib/walletApi'

interface WalletData {
  address: string
  balance: number
  staked: number
  transactions: any[]
}

export default function WalletOverview() {
  const [walletData, setWalletData] = useState<WalletData | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [address, setAddress] = useState('')

  const loadWalletData = async (walletAddress: string) => {
    if (!walletAddress) return
    
    setLoading(true)
    setError(null)
    
    try {
      const [balance, transactions] = await Promise.all([
        getWalletBalance(walletAddress),
        getWalletTransactions(walletAddress)
      ])
      
      setWalletData({
        address: walletAddress,
        balance: balance.balance || 0,
        staked: balance.staked || 0,
        transactions: transactions || []
      })
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load wallet data')
    } finally {
      setLoading(false)
    }
  }

  const handleAddressSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (address.trim()) {
      loadWalletData(address.trim())
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">Wallet Overview</h1>
        <Badge variant="success">Connected</Badge>
      </div>

      {/* Address Input */}
      <Card>
        <form onSubmit={handleAddressSubmit} className="space-y-4">
          <Field label="Wallet Address">
            <Input
              type="text"
              placeholder="Enter wallet address..."
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              className="font-mono"
            />
          </Field>
          <Button type="submit" disabled={loading || !address.trim()}>
            {loading ? 'Loading...' : 'Load Wallet'}
          </Button>
        </form>
      </Card>

      {/* Error Display */}
      {error && (
        <Card className="border-red-200 bg-red-50">
          <div className="text-red-800">
            <strong>Error:</strong> {error}
          </div>
        </Card>
      )}

      {/* Wallet Data */}
      {walletData && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* Balance Card */}
          <Card>
            <h3 className="text-lg font-semibold mb-4">Balance</h3>
            <div className="space-y-3">
              <div className="flex justify-between">
                <span className="text-gray-600">Available:</span>
                <span className="font-mono font-semibold">
                  {walletData.balance.toLocaleString()} IPPAN
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Staked:</span>
                <span className="font-mono font-semibold">
                  {walletData.staked.toLocaleString()} IPPAN
                </span>
              </div>
              <div className="flex justify-between border-t pt-3">
                <span className="text-gray-600 font-semibold">Total:</span>
                <span className="font-mono font-bold text-lg">
                  {(walletData.balance + walletData.staked).toLocaleString()} IPPAN
                </span>
              </div>
            </div>
          </Card>

          {/* Address Card */}
          <Card>
            <h3 className="text-lg font-semibold mb-4">Wallet Address</h3>
            <div className="space-y-3">
              <div className="break-all font-mono text-sm bg-gray-100 p-3 rounded">
                {walletData.address}
              </div>
              <Button 
                variant="outline" 
                size="sm"
                onClick={() => navigator.clipboard.writeText(walletData.address)}
              >
                Copy Address
              </Button>
            </div>
          </Card>
        </div>
      )}

      {/* Recent Transactions */}
      {walletData && walletData.transactions.length > 0 && (
        <Card>
          <h3 className="text-lg font-semibold mb-4">Recent Transactions</h3>
          <div className="space-y-3">
            {walletData.transactions.slice(0, 5).map((tx, index) => (
              <div key={index} className="flex justify-between items-center p-3 bg-gray-50 rounded">
                <div>
                  <div className="font-mono text-sm">{tx.hash}</div>
                  <div className="text-sm text-gray-600">
                    {new Date(tx.timestamp).toLocaleString()}
                  </div>
                </div>
                <div className="text-right">
                  <div className={`font-semibold ${tx.amount > 0 ? 'text-green-600' : 'text-red-600'}`}>
                    {tx.amount > 0 ? '+' : ''}{tx.amount} IPPAN
                  </div>
                  <Badge variant={tx.status === 'confirmed' ? 'success' : 'warning'}>
                    {tx.status}
                  </Badge>
                </div>
              </div>
            ))}
          </div>
        </Card>
      )}

      {/* Quick Actions */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Quick Actions</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <Button className="w-full">
            💸 Send Payment
          </Button>
          <Button className="w-full" variant="outline">
            🛡️ Stake Tokens
          </Button>
          <Button className="w-full" variant="outline">
            🌐 Manage Domains
          </Button>
        </div>
      </Card>
    </div>
  )
}
