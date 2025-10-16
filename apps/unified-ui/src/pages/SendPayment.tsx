import { useState } from 'react'
import { Card, Button, Field, Input } from '../components/UI'

interface TransactionForm {
  to: string
  amount: string
  memo: string
  fee: string
}

export default function SendPayment() {
  const [form, setForm] = useState<TransactionForm>({
    to: '',
    amount: '',
    memo: '',
    fee: '0.001'
  })
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<string | null>(null)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError(null)
    setSuccess(null)

    try {
      // Validate form
      if (!form.to || !form.amount) {
        throw new Error('Recipient address and amount are required')
      }

      const amount = parseFloat(form.amount)
      if (isNaN(amount) || amount <= 0) {
        throw new Error('Amount must be a positive number')
      }

      // Simulate transaction submission
      await new Promise(resolve => setTimeout(resolve, 2000))
      
      setSuccess(`Transaction submitted successfully! Amount: ${amount} IPPAN to ${form.to}`)
      
      // Reset form
      setForm({
        to: '',
        amount: '',
        memo: '',
        fee: '0.001'
      })
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Transaction failed')
    } finally {
      setLoading(false)
    }
  }

  const handleInputChange = (field: keyof TransactionForm, value: string) => {
    setForm(prev => ({ ...prev, [field]: value }))
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">Send Payment</h1>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Transaction Form */}
        <Card>
          <h3 className="text-lg font-semibold mb-4">Transaction Details</h3>
          <form onSubmit={handleSubmit} className="space-y-4">
            <Field label="Recipient Address" required>
              <Input
                type="text"
                placeholder="Enter recipient address..."
                value={form.to}
                onChange={(e) => handleInputChange('to', e.target.value)}
                className="font-mono"
              />
            </Field>

            <Field label="Amount (IPPAN)" required>
              <Input
                type="number"
                step="0.000001"
                placeholder="0.000000"
                value={form.amount}
                onChange={(e) => handleInputChange('amount', e.target.value)}
              />
            </Field>

            <Field label="Transaction Fee (IPPAN)">
              <Input
                type="number"
                step="0.000001"
                placeholder="0.001"
                value={form.fee}
                onChange={(e) => handleInputChange('fee', e.target.value)}
              />
            </Field>

            <Field label="Memo (Optional)">
              <Input
                type="text"
                placeholder="Add a note..."
                value={form.memo}
                onChange={(e) => handleInputChange('memo', e.target.value)}
              />
            </Field>

            <Button 
              type="submit" 
              disabled={loading || !form.to || !form.amount}
              className="w-full"
            >
              {loading ? 'Sending...' : 'Send Transaction'}
            </Button>
          </form>
        </Card>

        {/* Transaction Preview */}
        <Card>
          <h3 className="text-lg font-semibold mb-4">Transaction Preview</h3>
          <div className="space-y-3">
            <div className="flex justify-between">
              <span className="text-gray-600">To:</span>
              <span className="font-mono text-sm break-all">
                {form.to || 'Not specified'}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600">Amount:</span>
              <span className="font-semibold">
                {form.amount || '0'} IPPAN
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600">Fee:</span>
              <span className="font-semibold">
                {form.fee} IPPAN
              </span>
            </div>
            <div className="flex justify-between border-t pt-3">
              <span className="text-gray-600 font-semibold">Total:</span>
              <span className="font-bold text-lg">
                {(parseFloat(form.amount || '0') + parseFloat(form.fee)).toFixed(6)} IPPAN
              </span>
            </div>
            {form.memo && (
              <div className="mt-3 p-3 bg-gray-50 rounded">
                <div className="text-sm text-gray-600">Memo:</div>
                <div className="text-sm">{form.memo}</div>
              </div>
            )}
          </div>
        </Card>
      </div>

      {/* Status Messages */}
      {error && (
        <Card className="border-red-200 bg-red-50">
          <div className="text-red-800">
            <strong>Error:</strong> {error}
          </div>
        </Card>
      )}

      {success && (
        <Card className="border-green-200 bg-green-50">
          <div className="text-green-800">
            <strong>Success:</strong> {success}
          </div>
        </Card>
      )}

      {/* Quick Actions */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Quick Actions</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Button 
            variant="outline" 
            onClick={() => navigator.clipboard.readText().then(setForm)}
          >
            📋 Paste from Clipboard
          </Button>
          <Button 
            variant="outline"
            onClick={() => setForm(prev => ({ ...prev, to: '', amount: '', memo: '' }))}
          >
            🗑️ Clear Form
          </Button>
        </div>
      </Card>
    </div>
  )
}
