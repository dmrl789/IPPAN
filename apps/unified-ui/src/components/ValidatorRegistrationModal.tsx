import { useState } from 'react'
import { Modal, Button, Field, Input, LoadingSpinner } from './UI'
import { useToast } from './Toast'

interface ValidatorRegistrationModalProps {
  isOpen: boolean
  onClose: () => void
  onSuccess: () => void
}

interface ValidatorRegistrationData {
  nodeId: string
  stakeAmount: string
  publicKey: string
  commissionRate: string
  moniker: string
  website: string
  description: string
}

export default function ValidatorRegistrationModal({ 
  isOpen, 
  onClose, 
  onSuccess 
}: ValidatorRegistrationModalProps) {
  const [formData, setFormData] = useState<ValidatorRegistrationData>({
    nodeId: '',
    stakeAmount: '',
    publicKey: '',
    commissionRate: '5.0',
    moniker: '',
    website: '',
    description: ''
  })
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const { error: showError } = useToast()

  const handleInputChange = (field: keyof ValidatorRegistrationData, value: string) => {
    setFormData(prev => ({ ...prev, [field]: value }))
    setError(null)
  }

  const validateForm = (): string | null => {
    if (!formData.nodeId.trim()) return 'Node ID is required'
    if (!formData.stakeAmount.trim()) return 'Stake amount is required'
    if (!formData.publicKey.trim()) return 'Public key is required'
    if (!formData.moniker.trim()) return 'Validator name is required'
    
    // Validate node ID format (should be alphanumeric with underscores)
    if (!/^[a-zA-Z0-9_]+$/.test(formData.nodeId)) {
      return 'Node ID must contain only letters, numbers, and underscores'
    }
    
    const stakeAmount = parseFloat(formData.stakeAmount)
    if (isNaN(stakeAmount) || stakeAmount < 10000) {
      return 'Minimum stake amount is 10,000 IPPAN'
    }
    
    const commissionRate = parseFloat(formData.commissionRate)
    if (isNaN(commissionRate) || commissionRate < 0 || commissionRate > 100) {
      return 'Commission rate must be between 0% and 100%'
    }
    
    // Basic public key validation (should start with 0x and be hex)
    if (!formData.publicKey.startsWith('0x') || !/^0x[0-9a-fA-F]+$/.test(formData.publicKey)) {
      return 'Public key must be a valid hexadecimal string starting with 0x'
    }
    
    // Validate website URL if provided
    if (formData.website && formData.website.trim()) {
      try {
        new URL(formData.website)
      } catch {
        return 'Please enter a valid website URL'
      }
    }
    
    return null
  }

  const handleSubmit = async () => {
    const validationError = validateForm()
    if (validationError) {
      setError(validationError)
      return
    }

    setIsSubmitting(true)
    setError(null)

    try {
      const response = await fetch('/api/v1/validators/register', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          node_id: formData.nodeId,
          stake_amount: parseFloat(formData.stakeAmount),
          public_key: formData.publicKey,
          commission_rate: parseFloat(formData.commissionRate) / 100,
          moniker: formData.moniker,
          website: formData.website,
          description: formData.description,
        }),
      })

      if (!response.ok) {
        const errorData = await response.json()
        throw new Error(errorData.message || 'Failed to register validator')
      }

      const result = await response.json()
      console.log('Validator registration successful:', result)
      
      // Reset form
      setFormData({
        nodeId: '',
        stakeAmount: '',
        publicKey: '',
        commissionRate: '5.0',
        moniker: '',
        website: '',
        description: ''
      })
      
      onSuccess()
      onClose()
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'An unexpected error occurred'
      setError(errorMessage)
      showError(
        'Registration Failed',
        errorMessage
      )
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleClose = () => {
    if (!isSubmitting) {
      setError(null)
      onClose()
    }
  }

  return (
    <Modal isOpen={isOpen} onClose={handleClose} title="Become a Validator">
      <div className="space-y-4">
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
            {error}
          </div>
        )}

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Field label="Validator Name (Moniker)" required>
            <Input
              type="text"
              placeholder="My Validator"
              value={formData.moniker}
              onChange={(e) => handleInputChange('moniker', e.target.value)}
              disabled={isSubmitting}
            />
          </Field>

          <Field label="Node ID" required>
            <Input
              type="text"
              placeholder="node_1234567890abcdef"
              value={formData.nodeId}
              onChange={(e) => handleInputChange('nodeId', e.target.value)}
              disabled={isSubmitting}
            />
          </Field>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Field label="Stake Amount (IPPAN)" required>
            <Input
              type="number"
              placeholder="10000"
              min="10000"
              value={formData.stakeAmount}
              onChange={(e) => handleInputChange('stakeAmount', e.target.value)}
              disabled={isSubmitting}
            />
            <p className="text-xs text-gray-500 mt-1">
              Minimum: 10,000 IPPAN
            </p>
          </Field>

          <Field label="Commission Rate (%)" required>
            <Input
              type="number"
              placeholder="5.0"
              min="0"
              max="100"
              step="0.1"
              value={formData.commissionRate}
              onChange={(e) => handleInputChange('commissionRate', e.target.value)}
              disabled={isSubmitting}
            />
          </Field>
        </div>

        <Field label="Public Key" required>
          <Input
            type="text"
            placeholder="0x1234567890abcdef1234567890abcdef12345678"
            value={formData.publicKey}
            onChange={(e) => handleInputChange('publicKey', e.target.value)}
            disabled={isSubmitting}
          />
          <p className="text-xs text-gray-500 mt-1">
            Enter your validator's public key (hexadecimal format)
          </p>
        </Field>

        <Field label="Website (Optional)">
          <Input
            type="url"
            placeholder="https://myvalidator.com"
            value={formData.website}
            onChange={(e) => handleInputChange('website', e.target.value)}
            disabled={isSubmitting}
          />
        </Field>

        <Field label="Description (Optional)">
          <textarea
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            placeholder="Describe your validator and its benefits..."
            rows={3}
            value={formData.description}
            onChange={(e) => handleInputChange('description', e.target.value)}
            disabled={isSubmitting}
          />
        </Field>

        <div className="bg-blue-50 border border-blue-200 rounded-md p-4">
          <h4 className="font-medium text-blue-900 mb-2">Validator Requirements</h4>
                      <ul className="text-sm text-blue-800 space-y-1">
              <li>• Minimum stake: 10,000 IPPAN</li>
              <li>• Must maintain high uptime (&gt;95%)</li>
              <li>• Must participate in consensus rounds</li>
              <li>• Commission rate: 0-100%</li>
              <li>• Validator can be slashed for malicious behavior</li>
            </ul>
        </div>

        <div className="flex justify-end space-x-3 pt-4">
          <Button
            variant="secondary"
            onClick={handleClose}
            disabled={isSubmitting}
          >
            Cancel
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={isSubmitting}
            className="min-w-[120px]"
          >
            {isSubmitting ? (
              <>
                <LoadingSpinner />
                <span className="ml-2">Registering...</span>
              </>
            ) : (
              'Register Validator'
            )}
          </Button>
        </div>
      </div>
    </Modal>
  )
}
