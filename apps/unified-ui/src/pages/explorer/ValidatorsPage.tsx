import { useEffect, useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Card, Button, Badge, LoadingSpinner } from '../../components/UI'
import ValidatorRegistrationModal from '../../components/ValidatorRegistrationModal'
import { ToastContainer, useToast } from '../../components/Toast'
import { getConsensusStats, getValidators, ValidatorInfo } from '../../lib/api'

export default function ValidatorsPage() {
  const [isRegistrationModalOpen, setIsRegistrationModalOpen] = useState(false)
  const { toasts, removeToast, success, error: showError } = useToast()

  const validatorsQuery = useQuery({
    queryKey: ['validators'],
    queryFn: getValidators,
  })

  const consensusQuery = useQuery({
    queryKey: ['consensus'],
    queryFn: getConsensusStats,
  })

  useEffect(() => {
    if (!validatorsQuery.isError) {
      return
    }

    const message =
      validatorsQuery.error instanceof Error
        ? validatorsQuery.error.message
        : 'Unable to load validators'

    showError('Failed to load validators', message)
  }, [validatorsQuery.isError, validatorsQuery.error, showError])

  const validators: ValidatorInfo[] = validatorsQuery.data ?? []

  const numberFormatter = useMemo(
    () => new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 }),
    []
  )

  const totals = useMemo(() => {
    if (!validators.length) {
      return {
        active: 0,
        inactive: 0,
        totalStake: 0,
        averageStake: 0,
      }
    }

    const active = validators.filter(v => v.is_active).length
    const inactive = validators.length - active
    const totalStake = validators.reduce((sum, v) => sum + v.stake_amount, 0)

    return {
      active,
      inactive,
      totalStake,
      averageStake: totalStake / validators.length,
    }
  }, [validators])

  const formatStake = (amount: number) => `${numberFormatter.format(amount)} IPN`

  const handleRegistrationSuccess = () => {
    success(
      'Validator Registration Successful!',
      'Your validator registration has been submitted and is being processed. You will be notified once it becomes active.',
      8000
    )

    void validatorsQuery.refetch()
  }

  if (validatorsQuery.isLoading) {
    return (
      <div className="flex justify-center items-center h-64">
        <LoadingSpinner />
      </div>
    )
  }

  if (validatorsQuery.isError) {
    const message =
      validatorsQuery.error instanceof Error
        ? validatorsQuery.error.message
        : 'Unable to load validators'

    return (
      <div className="space-y-4">
        <h1 className="text-2xl font-bold text-gray-900">Validators</h1>
        <Card>
          <div className="text-sm text-red-600">{message}</div>
        </Card>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold text-gray-900">Validators</h1>
        <Badge variant="success">Live Network</Badge>
      </div>

      <Card title="Validator Set">
        {validators.length === 0 ? (
          <div className="text-sm text-gray-600">No validators are currently registered.</div>
        ) : (
          <div className="space-y-4">
            {validators.map((validator) => (
              <div key={validator.node_id} className="p-4 border rounded-lg">
                <div className="flex justify-between items-start">
                  <div className="flex-1">
                    <div className="flex items-center space-x-2 mb-2">
                      <h3 className="font-semibold">{validator.node_id.slice(0, 12)}…</h3>
                      <Badge variant={validator.is_active ? 'success' : 'warning'}>
                        {validator.is_active ? 'active' : 'inactive'}
                      </Badge>
                    </div>
                    <div className="text-sm text-gray-600 space-y-1">
                      <div>Node ID: {validator.node_id}</div>
                      <div>Address: {validator.address}</div>
                    </div>
                  </div>
                  <div className="text-right text-sm text-gray-600">
                    <div className="text-lg font-bold text-green-600">{formatStake(validator.stake_amount)}</div>
                    <div>Total Stake</div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <Card title="Network Statistics">
          <div className="space-y-4">
            <div className="text-center">
              <div className="text-3xl font-bold text-blue-600">{validators.length}</div>
              <div className="text-sm text-gray-600">Total Validators</div>
            </div>
            <div className="text-center">
              <div className="text-3xl font-bold text-green-600">{totals.active}</div>
              <div className="text-sm text-gray-600">Active</div>
            </div>
            <div className="text-center">
              <div className="text-3xl font-bold text-yellow-600">{totals.inactive}</div>
              <div className="text-sm text-gray-600">Inactive</div>
            </div>
          </div>
        </Card>

        <Card title="Consensus Overview">
          {consensusQuery.isLoading ? (
            <div className="flex justify-center py-6">
              <LoadingSpinner />
            </div>
          ) : consensusQuery.isError ? (
            <div className="text-sm text-red-600">Unable to load consensus details</div>
          ) : consensusQuery.data ? (
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700">Current Round</label>
                <div className="text-2xl font-bold text-blue-600">{consensusQuery.data.current_round.toLocaleString()}</div>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700">Latest Block Height</label>
                <div className="text-2xl font-bold text-purple-600">{consensusQuery.data.block_height.toLocaleString()}</div>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700">Consensus Status</label>
                <div className="text-2xl font-bold text-green-600 capitalize">{consensusQuery.data.consensus_status}</div>
              </div>
            </div>
          ) : (
            <div className="text-sm text-gray-600">Consensus data unavailable.</div>
          )}
        </Card>

        <Card title="Staking Overview">
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700">Total Staked</label>
              <div className="text-2xl font-bold text-green-600">{formatStake(totals.totalStake)}</div>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700">Average Stake</label>
              <div className="text-2xl font-bold text-orange-600">{formatStake(totals.averageStake)}</div>
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
