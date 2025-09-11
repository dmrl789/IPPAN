import { useState } from 'react'
import { Card, Button, Field, Input, Badge, LoadingSpinner } from '../components/UI'

export default function PaymentsPage() {
  const [recipient, setRecipient] = useState('')
  const [amount, setAmount] = useState('')
  const [memo, setMemo] = useState('')
  const [isProcessing, setIsProcessing] = useState(false)

  const handleSendPayment = async () => {
    setIsProcessing(true)
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 2000))
    setIsProcessing(false)
    // Reset form
    setRecipient('')
    setAmount('')
    setMemo('')
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold text-gray-900">Payments & M2M</h1>
        <Badge variant="success">Connected</Badge>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Send Payment */}
        <Card title="Send Payment">
          <div className="space-y-4">
            <Field label="Recipient Address">
              <Input
                placeholder="0x..."
                value={recipient}
                onChange={(e) => setRecipient(e.target.value)}
              />
            </Field>
            <Field label="Amount (IPPAN)">
              <Input
                type="number"
                placeholder="0.00"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
              />
            </Field>
            <Field label="Memo (Optional)">
              <Input
                placeholder="Payment description"
                value={memo}
                onChange={(e) => setMemo(e.target.value)}
              />
            </Field>
            <Button 
              onClick={handleSendPayment}
              disabled={!recipient || !amount || isProcessing}
              className="w-full"
            >
              {isProcessing ? <LoadingSpinner /> : 'Send Payment'}
            </Button>
          </div>
        </Card>

        {/* M2M Channels */}
        <Card title="M2M Payment Channels">
          <div className="space-y-4">
            <div className="p-3 bg-blue-50 rounded border">
              <div className="flex justify-between items-center">
                <div>
                  <div className="font-medium">Channel #1</div>
                                     <div className="text-sm text-gray-600">With: iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X</div>
                </div>
                <Badge variant="success">Active</Badge>
              </div>
              <div className="mt-2 text-sm">
                <span className="text-gray-600">Balance: </span>
                <span className="font-medium">150 IPPAN</span>
              </div>
            </div>
            
            <div className="p-3 bg-gray-50 rounded border">
              <div className="flex justify-between items-center">
                <div>
                  <div className="font-medium">Channel #2</div>
                                     <div className="text-sm text-gray-600">With: iDLZ4d490pJHpsL2PDoXTDA8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0X</div>
                </div>
                <Badge variant="warning">Pending</Badge>
              </div>
              <div className="mt-2 text-sm">
                <span className="text-gray-600">Balance: </span>
                <span className="font-medium">75 IPPAN</span>
              </div>
            </div>

            <Button className="w-full bg-green-600 hover:bg-green-700">
              Create New Channel
            </Button>
          </div>
        </Card>
      </div>

      {/* Recent Transactions */}
      <Card title="Recent Transactions">
        <div className="space-y-3">
          <div className="flex justify-between items-center p-3 bg-gray-50 rounded">
            <div>
              <div className="font-medium">Payment Sent</div>
                             <div className="text-sm text-gray-600">To: iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X</div>
              <div className="text-xs text-gray-500">Memo: Lunch payment</div>
            </div>
            <div className="text-right">
              <div className="font-medium text-red-600">-25.50 IPPAN</div>
              <div className="text-sm text-gray-600">2 hours ago</div>
              <Badge variant="success" className="mt-1">Confirmed</Badge>
            </div>
          </div>

          <div className="flex justify-between items-center p-3 bg-gray-50 rounded">
            <div>
              <div className="font-medium">Payment Received</div>
                             <div className="text-sm text-gray-600">From: iDLZ4d490pJHpsL2PDoXTDA8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0X</div>
              <div className="text-xs text-gray-500">Memo: Project payment</div>
            </div>
            <div className="text-right">
              <div className="font-medium text-green-600">+500.00 IPPAN</div>
              <div className="text-sm text-gray-600">1 day ago</div>
              <Badge variant="success" className="mt-1">Confirmed</Badge>
            </div>
          </div>

          <div className="flex justify-between items-center p-3 bg-gray-50 rounded">
            <div>
              <div className="font-medium">M2M Channel Opened</div>
              <div className="text-sm text-gray-600">With: 0x5678...9abc</div>
              <div className="text-xs text-gray-500">Initial deposit: 100 IPPAN</div>
            </div>
            <div className="text-right">
              <div className="font-medium text-blue-600">Channel Created</div>
              <div className="text-sm text-gray-600">3 days ago</div>
              <Badge variant="default" className="mt-1">Active</Badge>
            </div>
          </div>
        </div>
      </Card>

      {/* Payment Statistics */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <Card title="This Month">
          <div className="text-center">
            <div className="text-2xl font-bold text-green-600">+1,250 IPPAN</div>
            <div className="text-sm text-gray-600">Received</div>
          </div>
        </Card>
        <Card title="This Month">
          <div className="text-center">
            <div className="text-2xl font-bold text-red-600">-450 IPPAN</div>
            <div className="text-sm text-gray-600">Sent</div>
          </div>
        </Card>
        <Card title="Active Channels">
          <div className="text-center">
            <div className="text-2xl font-bold text-blue-600">3</div>
            <div className="text-sm text-gray-600">M2M Channels</div>
          </div>
        </Card>
      </div>
    </div>
  )
}
