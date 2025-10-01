import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Card, Button, Field, Input, Badge, LoadingSpinner } from '../components/UI'
import { getWalletBalance } from '../lib/api'

export default function WalletPage() {
  const [address, setAddress] = useState('')
  const [showConnect, setShowConnect] = useState(false)

  const { data: balance, isLoading, error } = useQuery({
    queryKey: ['wallet-balance', address],
    queryFn: () => getWalletBalance(address),
    enabled: !!address,
  })

  const staked = balance?.staked ?? balance?.staked_amount ?? 0
  const rewards = balance?.rewards ?? 0
  const available = balance?.balance ?? 0
  const total = available + staked + rewards

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold text-gray-900">Wallet Overview</h1>
        <Button onClick={() => setShowConnect(true)}>Connect Wallet</Button>
      </div>

      {!address ? (
        <Card title="Connect Your Wallet">
          <div className="space-y-4">
            <p className="text-gray-600">
              Connect your wallet to view balances, manage transactions, and access IPPAN features.
            </p>
            <Field label="Wallet Address">
              <Input
                placeholder="Enter your wallet address (i...)"
                value={address}
                onChange={(e) => setAddress(e.target.value)}
              />
            </Field>
            <Button 
              onClick={() => {
                setAddress("i0000000000000000000000000000000000000000000000000000000000000000");
                setShowConnect(true);
              }}
              className="bg-blue-600 hover:bg-blue-700"
            >
              Connect Demo Wallet
            </Button>
          </div>
        </Card>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {/* Balance Card */}
          <Card title="Account Balance">
            {isLoading ? (
              <LoadingSpinner />
            ) : error ? (
              <div className="text-red-600">Error loading balance</div>
            ) : balance ? (
              <div className="space-y-3">
                <div className="flex justify-between">
                  <span className="text-gray-600">Available:</span>
                  <span className="font-semibold">{available} IPPAN</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-600">Staked:</span>
                  <span className="font-semibold">{staked} IPPAN</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-600">Rewards:</span>
                  <span className="font-semibold text-green-600">{rewards} IPPAN</span>
                </div>
                <div className="pt-2 border-t">
                  <div className="flex justify-between">
                    <span className="font-semibold">Total:</span>
                    <span className="font-bold text-lg">{total} IPPAN</span>
                  </div>
                </div>
              </div>
            ) : (
              <div className="text-gray-500">No balance data</div>
            )}
          </Card>

          {/* Quick Actions */}
          <Card title="Quick Actions">
            <div className="space-y-3">
              <Button className="w-full">Send Transaction</Button>
              <Button className="w-full bg-green-600 hover:bg-green-700">Stake Tokens</Button>
              <Button className="w-full bg-purple-600 hover:bg-purple-700">Claim Rewards</Button>
              <Button className="w-full bg-gray-600 hover:bg-gray-700">View History</Button>
            </div>
          </Card>

          {/* Network Status */}
          <Card title="Network Status">
            <div className="space-y-3">
              <div className="flex justify-between items-center">
                <span className="text-gray-600">Connection:</span>
                <Badge variant="success">Connected</Badge>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-gray-600">Network:</span>
                <Badge variant="default">IPPAN Mainnet</Badge>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-gray-600">Block Height:</span>
                <span className="font-mono text-sm">1,234,567</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-gray-600">Gas Price:</span>
                <span className="font-mono text-sm">0.001 IPPAN</span>
              </div>
            </div>
          </Card>
        </div>
      )}

      {/* Recent Activity */}
      <Card title="Recent Activity">
        <div className="space-y-3">
          <div className="flex justify-between items-center p-3 bg-gray-50 rounded">
            <div>
              <div className="font-medium">Sent Payment</div>
              <div className="text-sm text-gray-600">To: i9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4</div>
            </div>
            <div className="text-right">
              <div className="font-medium text-red-600">-50 IPPAN</div>
              <div className="text-sm text-gray-600">2 hours ago</div>
            </div>
          </div>
          <div className="flex justify-between items-center p-3 bg-gray-50 rounded">
            <div>
              <div className="font-medium">Staking Reward</div>
              <div className="text-sm text-gray-600">Validator rewards</div>
            </div>
            <div className="text-right">
              <div className="font-medium text-green-600">+12.5 IPPAN</div>
              <div className="text-sm text-gray-600">1 day ago</div>
            </div>
          </div>
          <div className="flex justify-between items-center p-3 bg-gray-50 rounded">
            <div>
              <div className="font-medium">Domain Registration</div>
              <div className="text-sm text-gray-600">example.ippan</div>
            </div>
            <div className="text-right">
              <div className="font-medium text-red-600">-100 IPPAN</div>
              <div className="text-sm text-gray-600">3 days ago</div>
            </div>
          </div>
        </div>
      </Card>
    </div>
  )
}
