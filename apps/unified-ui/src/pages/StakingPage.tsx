import { useState } from 'react'
import { Card, Button, Field, Input, Badge, LoadingSpinner } from '../components/UI'

export default function StakingPage() {
  const [stakeAmount, setStakeAmount] = useState('')
  const [isProcessing, setIsProcessing] = useState(false)

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold text-gray-900">Staking & Validator</h1>
        <Badge variant="success">Validator Active</Badge>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card title="Stake Tokens">
          <div className="space-y-4">
            <Field label="Amount to Stake (IPPAN)">
              <Input
                type="number"
                placeholder="0.00"
                value={stakeAmount}
                onChange={(e) => setStakeAmount(e.target.value)}
              />
            </Field>
            <Button 
              onClick={() => setIsProcessing(true)}
              disabled={!stakeAmount || isProcessing}
              className="w-full"
            >
              {isProcessing ? <LoadingSpinner /> : 'Stake Tokens'}
            </Button>
          </div>
        </Card>

        <Card title="Validator Status">
          <div className="space-y-3">
            <div className="flex justify-between">
              <span>Total Staked:</span>
              <span className="font-semibold">1,500 IPPAN</span>
            </div>
            <div className="flex justify-between">
              <span>Rewards Earned:</span>
              <span className="font-semibold text-green-600">125.50 IPPAN</span>
            </div>
            <div className="flex justify-between">
              <span>Validator Rank:</span>
              <span className="font-semibold">#42</span>
            </div>
            <div className="flex justify-between">
              <span>Uptime:</span>
              <span className="font-semibold text-green-600">99.8%</span>
            </div>
          </div>
        </Card>
      </div>

      <Card title="Network Statistics">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="text-center">
            <div className="text-2xl font-bold text-blue-600">1,234</div>
            <div className="text-sm text-gray-600">Active Validators</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold text-green-600">45.2M</div>
            <div className="text-sm text-gray-600">Total Staked</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold text-purple-600">12.5%</div>
            <div className="text-sm text-gray-600">APY</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold text-orange-600">2.3s</div>
            <div className="text-sm text-gray-600">Avg Block Time</div>
          </div>
        </div>
      </Card>
    </div>
  )
}
