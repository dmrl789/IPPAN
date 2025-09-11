import { useState, useEffect } from 'react'
import { Card, Button, Badge, LoadingSpinner } from '../../components/UI'
import ValidatorRegistrationModal from '../../components/ValidatorRegistrationModal'
import { ToastContainer, useToast } from '../../components/Toast'

interface Validator {
  address: string
  name: string
  status: 'active' | 'inactive' | 'slashed'
  stakedAmount: string
  commission: number
  uptime: number
  blocksProduced: number
  lastBlock: string
  performance: number
}

export default function ValidatorsPage() {
  const [validators, setValidators] = useState<Validator[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [isRegistrationModalOpen, setIsRegistrationModalOpen] = useState(false)
  const { toasts, removeToast, success, error } = useToast()

  useEffect(() => {
    const mockValidators: Validator[] = [
      {
        address: '0xValidator1...',
        name: 'Validator Alpha',
        status: 'active',
        stakedAmount: '50,000 IPPAN',
        commission: 5.0,
        uptime: 99.8,
        blocksProduced: 1234,
        lastBlock: new Date().toISOString(),
        performance: 98.5
      },
      {
        address: '0xValidator2...',
        name: 'Validator Beta',
        status: 'active',
        stakedAmount: '45,000 IPPAN',
        commission: 3.5,
        uptime: 99.9,
        blocksProduced: 1189,
        lastBlock: new Date(Date.now() - 300000).toISOString(),
        performance: 99.2
      },
      {
        address: '0xValidator3...',
        name: 'Validator Gamma',
        status: 'inactive',
        stakedAmount: '30,000 IPPAN',
        commission: 4.0,
        uptime: 85.2,
        blocksProduced: 567,
        lastBlock: new Date(Date.now() - 3600000).toISOString(),
        performance: 75.8
      }
    ]

    setValidators(mockValidators)
    setIsLoading(false)
  }, [])

  const handleRegistrationSuccess = () => {
    success(
      'Validator Registration Successful!',
      'Your validator registration has been submitted and is being processed. You will be notified once it becomes active.',
      8000
    )
    
    // In a real app, you might want to refresh the validators list here
    // fetchValidators()
  }

  if (isLoading) {
    return (
      <div className="flex justify-center items-center h-64">
        <LoadingSpinner />
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold text-gray-900">Validators</h1>
        <Badge variant="success">Live Network</Badge>
      </div>

      <Card title="Active Validators">
        <div className="space-y-4">
          {validators.map((validator) => (
            <div key={validator.address} className="p-4 border rounded-lg">
              <div className="flex justify-between items-start">
                <div className="flex-1">
                  <div className="flex items-center space-x-2 mb-2">
                    <h3 className="font-semibold">{validator.name}</h3>
                    <Badge variant={validator.status === 'active' ? 'success' : validator.status === 'inactive' ? 'warning' : 'error'}>
                      {validator.status}
                    </Badge>
                  </div>
                  <div className="text-sm text-gray-600 space-y-1">
                    <div>Address: {validator.address}</div>
                    <div>Staked: {validator.stakedAmount}</div>
                    <div>Commission: {validator.commission}%</div>
                    <div>Uptime: {validator.uptime}%</div>
                  </div>
                </div>
                <div className="text-right text-sm text-gray-600">
                  <div className="text-lg font-bold">{validator.performance}%</div>
                  <div>Performance</div>
                  <div>{validator.blocksProduced} blocks</div>
                  <div>{new Date(validator.lastBlock).toLocaleTimeString()}</div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </Card>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <Card title="Network Statistics">
          <div className="space-y-4">
            <div className="text-center">
              <div className="text-3xl font-bold text-blue-600">{validators.length}</div>
              <div className="text-sm text-gray-600">Total Validators</div>
            </div>
            <div className="text-center">
              <div className="text-3xl font-bold text-green-600">
                {validators.filter(v => v.status === 'active').length}
              </div>
              <div className="text-sm text-gray-600">Active</div>
            </div>
            <div className="text-center">
              <div className="text-3xl font-bold text-yellow-600">
                {validators.filter(v => v.status === 'inactive').length}
              </div>
              <div className="text-sm text-gray-600">Inactive</div>
            </div>
          </div>
        </Card>

        <Card title="Performance Metrics">
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700">Average Uptime</label>
              <div className="text-2xl font-bold text-green-600">
                {(validators.reduce((sum, v) => sum + v.uptime, 0) / validators.length).toFixed(1)}%
              </div>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700">Average Performance</label>
              <div className="text-2xl font-bold text-blue-600">
                {(validators.reduce((sum, v) => sum + v.performance, 0) / validators.length).toFixed(1)}%
              </div>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700">Total Blocks</label>
              <div className="text-2xl font-bold text-purple-600">
                {validators.reduce((sum, v) => sum + v.blocksProduced, 0).toLocaleString()}
              </div>
            </div>
          </div>
        </Card>

        <Card title="Staking Overview">
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700">Total Staked</label>
              <div className="text-2xl font-bold text-green-600">
                {validators.reduce((sum, v) => sum + parseFloat(v.stakedAmount.replace(/[^\d.]/g, '')), 0).toLocaleString()} IPPAN
              </div>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700">Average Commission</label>
              <div className="text-2xl font-bold text-orange-600">
                {(validators.reduce((sum, v) => sum + v.commission, 0) / validators.length).toFixed(1)}%
              </div>
            </div>
            <Button 
              className="w-full"
              onClick={() => setIsRegistrationModalOpen(true)}
            >
              Become Validator
            </Button>
          </div>
        </Card>
      </div>

      <ValidatorRegistrationModal
        isOpen={isRegistrationModalOpen}
        onClose={() => setIsRegistrationModalOpen(false)}
        onSuccess={handleRegistrationSuccess}
      />

      <ToastContainer toasts={toasts} onRemove={removeToast} />
    </div>
  )
}
