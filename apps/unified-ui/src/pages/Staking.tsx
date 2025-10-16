import { useState, useEffect } from 'react'
import { Card, Button, Field, Input, Badge } from '../components/UI'

interface StakingData {
  totalStaked: number
  activeStakes: Array<{
    id: string
    amount: number
    validator: string
    startDate: string
    endDate: string
    status: 'active' | 'pending' | 'completed'
  }>
  rewards: number
}

export default function Staking() {
  const [stakingData, setStakingData] = useState<StakingData | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  
  // Staking form
  const [stakeForm, setStakeForm] = useState({
    amount: '',
    validator: '',
    duration: '30'
  })

  const [validators] = useState([
    { id: 'validator-1', name: 'IPPAN Validator #1', commission: '5%', uptime: '99.8%' },
    { id: 'validator-2', name: 'IPPAN Validator #2', commission: '3%', uptime: '99.9%' },
    { id: 'validator-3', name: 'IPPAN Validator #3', commission: '4%', uptime: '99.7%' },
  ])

  useEffect(() => {
    loadStakingData()
  }, [])

  const loadStakingData = async () => {
    setLoading(true)
    try {
      // Simulate API call
      await new Promise(resolve => setTimeout(resolve, 1000))
      
      setStakingData({
        totalStaked: 1250.5,
        activeStakes: [
          {
            id: 'stake-1',
            amount: 500,
            validator: 'IPPAN Validator #1',
            startDate: '2024-01-15',
            endDate: '2024-02-15',
            status: 'active'
          },
          {
            id: 'stake-2',
            amount: 750.5,
            validator: 'IPPAN Validator #2',
            startDate: '2024-01-20',
            endDate: '2024-02-20',
            status: 'active'
          }
        ],
        rewards: 25.75
      })
    } catch (err) {
      setError('Failed to load staking data')
    } finally {
      setLoading(false)
    }
  }

  const handleStake = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError(null)

    try {
      const amount = parseFloat(stakeForm.amount)
      if (isNaN(amount) || amount <= 0) {
        throw new Error('Amount must be a positive number')
      }

      // Simulate staking
      await new Promise(resolve => setTimeout(resolve, 2000))
      
      // Reset form and reload data
      setStakeForm({ amount: '', validator: '', duration: '30' })
      await loadStakingData()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Staking failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">Staking</h1>
        <Badge variant="success">Active</Badge>
      </div>

      {/* Staking Overview */}
      {stakingData && (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <Card>
            <h3 className="text-lg font-semibold mb-4">Total Staked</h3>
            <div className="text-3xl font-bold text-blue-600">
              {stakingData.totalStaked.toLocaleString()} IPPAN
            </div>
            <div className="text-sm text-gray-600 mt-2">
              Across {stakingData.activeStakes.length} active stakes
            </div>
          </Card>

          <Card>
            <h3 className="text-lg font-semibold mb-4">Total Rewards</h3>
            <div className="text-3xl font-bold text-green-600">
              {stakingData.rewards.toLocaleString()} IPPAN
            </div>
            <div className="text-sm text-gray-600 mt-2">
              Earned from staking
            </div>
          </Card>

          <Card>
            <h3 className="text-lg font-semibold mb-4">APY</h3>
            <div className="text-3xl font-bold text-purple-600">
              12.5%
            </div>
            <div className="text-sm text-gray-600 mt-2">
              Current network APY
            </div>
          </Card>
        </div>
      )}

      {/* New Stake Form */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Stake Tokens</h3>
        <form onSubmit={handleStake} className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <Field label="Amount (IPPAN)" required>
              <Input
                type="number"
                step="0.000001"
                placeholder="0.000000"
                value={stakeForm.amount}
                onChange={(e) => setStakeForm(prev => ({ ...prev, amount: e.target.value }))}
              />
            </Field>

            <Field label="Duration (Days)" required>
              <Input
                type="number"
                placeholder="30"
                value={stakeForm.duration}
                onChange={(e) => setStakeForm(prev => ({ ...prev, duration: e.target.value }))}
              />
            </Field>
          </div>

          <Field label="Validator" required>
            <select
              className="w-full p-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              value={stakeForm.validator}
              onChange={(e) => setStakeForm(prev => ({ ...prev, validator: e.target.value }))}
            >
              <option value="">Select a validator...</option>
              {validators.map(validator => (
                <option key={validator.id} value={validator.id}>
                  {validator.name} - {validator.commission} commission - {validator.uptime} uptime
                </option>
              ))}
            </select>
          </Field>

          <Button 
            type="submit" 
            disabled={loading || !stakeForm.amount || !stakeForm.validator}
            className="w-full"
          >
            {loading ? 'Staking...' : 'Stake Tokens'}
          </Button>
        </form>
      </Card>

      {/* Active Stakes */}
      {stakingData && stakingData.activeStakes.length > 0 && (
        <Card>
          <h3 className="text-lg font-semibold mb-4">Active Stakes</h3>
          <div className="space-y-4">
            {stakingData.activeStakes.map(stake => (
              <div key={stake.id} className="p-4 border border-gray-200 rounded-lg">
                <div className="flex justify-between items-start mb-3">
                  <div>
                    <div className="font-semibold">{stake.validator}</div>
                    <div className="text-sm text-gray-600">
                      {stake.startDate} - {stake.endDate}
                    </div>
                  </div>
                  <Badge variant={stake.status === 'active' ? 'success' : 'warning'}>
                    {stake.status}
                  </Badge>
                </div>
                <div className="flex justify-between items-center">
                  <div className="text-2xl font-bold">
                    {stake.amount.toLocaleString()} IPPAN
                  </div>
                  <div className="space-x-2">
                    <Button size="sm" variant="outline">
                      View Details
                    </Button>
                    <Button size="sm" variant="outline">
                      Unstake
                    </Button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </Card>
      )}

      {/* Validators List */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Available Validators</h3>
        <div className="space-y-3">
          {validators.map(validator => (
            <div key={validator.id} className="flex justify-between items-center p-3 bg-gray-50 rounded">
              <div>
                <div className="font-semibold">{validator.name}</div>
                <div className="text-sm text-gray-600">
                  Commission: {validator.commission} • Uptime: {validator.uptime}
                </div>
              </div>
              <Button size="sm" variant="outline">
                Stake with this validator
              </Button>
            </div>
          ))}
        </div>
      </Card>

      {/* Error Display */}
      {error && (
        <Card className="border-red-200 bg-red-50">
          <div className="text-red-800">
            <strong>Error:</strong> {error}
          </div>
        </Card>
      )}
    </div>
  )
}
