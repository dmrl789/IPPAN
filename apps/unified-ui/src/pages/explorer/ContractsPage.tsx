import { useState, useEffect } from 'react'
import { Card, Button, Badge, LoadingSpinner, Field, Input } from '../../components/UI'

interface Contract {
  address: string
  name: string
  type: 'ai_marketplace' | 'staking' | 'governance' | 'custom'
  verified: boolean
  creator: string
  deployedAt: string
  transactions: number
  balance: string
  sourceCode?: string
}

export default function ContractsPage() {
  const [contracts, setContracts] = useState<Contract[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [selectedContract, setSelectedContract] = useState<Contract | null>(null)
  const [searchTerm, setSearchTerm] = useState('')

  useEffect(() => {
    const mockContracts: Contract[] = [
      {
        address: 'iDLZ4d490pJHpsL2PDoXTDA8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0X',
        name: 'AI Job Marketplace',
        type: 'ai_marketplace',
        verified: true,
                 creator: 'iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X',
        deployedAt: '2024-01-15T10:30:00Z',
        transactions: 156,
        balance: '0.00 IPPAN',
        sourceCode: '// AI Job Marketplace Contract\ncontract AIJobMarketplace { ... }'
      },
      {
        address: 'iDLZ4d490pJHpsL2PDoXTDA8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0X',
        name: 'Staking Pool',
        type: 'staking',
        verified: true,
                 creator: 'iDLZ4d490pJHpsL2PDoXTDA8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0X',
        deployedAt: '2024-02-20T14:15:00Z',
        transactions: 89,
        balance: '125,000.00 IPPAN'
      },
      {
        address: 'iDLZ4d490pJHpsL2PDoXTDA8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0X',
        name: 'Governance DAO',
        type: 'governance',
        verified: false,
        creator: '0x9876543210fedcba9876543210fedcba9876543210',
        deployedAt: '2024-03-10T09:45:00Z',
        transactions: 234,
        balance: '0.00 IPPAN'
      }
    ]

    setContracts(mockContracts)
    setIsLoading(false)
  }, [])

  const filteredContracts = contracts.filter(contract =>
    contract.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
    contract.address.toLowerCase().includes(searchTerm.toLowerCase())
  )

  const getTypeColor = (type: string) => {
    switch (type) {
      case 'ai_marketplace': return 'pink'
      case 'staking': return 'green'
      case 'governance': return 'purple'
      case 'custom': return 'blue'
      default: return 'gray'
    }
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
        <h1 className="text-2xl font-bold text-gray-900">Smart Contracts</h1>
        <Badge variant="success">Live Network</Badge>
      </div>

      <Card title="Search Contracts">
        <Field label="Contract Name or Address">
          <Input
            placeholder="Search contracts..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
          />
        </Field>
      </Card>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2">
          <Card title={`Deployed Contracts (${filteredContracts.length})`}>
            <div className="space-y-3">
              {filteredContracts.map((contract) => (
                <div
                  key={contract.address}
                  className={`p-4 border rounded-lg cursor-pointer transition-all hover:shadow-md ${
                    selectedContract?.address === contract.address ? 'border-blue-500 bg-blue-50' : 'border-gray-200'
                  }`}
                  onClick={() => setSelectedContract(contract)}
                >
                  <div className="flex justify-between items-start">
                    <div className="flex-1">
                      <div className="flex items-center space-x-2 mb-2">
                        <h3 className="font-semibold">{contract.name}</h3>
                        <Badge variant={getTypeColor(contract.type) as any}>
                          {contract.type.replace('_', ' ').toUpperCase()}
                        </Badge>
                        <Badge variant={contract.verified ? 'success' : 'warning'}>
                          {contract.verified ? 'Verified' : 'Unverified'}
                        </Badge>
                      </div>
                      <div className="text-sm text-gray-600 space-y-1">
                        <div>Address: {contract.address.substring(0, 16)}...</div>
                        <div>Creator: {contract.creator.substring(0, 16)}...</div>
                        <div>Transactions: {contract.transactions}</div>
                        <div>Balance: {contract.balance}</div>
                      </div>
                    </div>
                    <div className="text-right text-sm text-gray-600">
                      <div>{contract.balance}</div>
                      <div>{contract.transactions} txs</div>
                      <div>{new Date(contract.deployedAt).toLocaleDateString()}</div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </Card>
        </div>

        <div className="lg:col-span-1">
          <Card title="Contract Details">
            {selectedContract ? (
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700">Contract Name</label>
                  <div className="font-semibold">{selectedContract.name}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Address</label>
                  <div className="font-mono text-xs break-all">{selectedContract.address}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Type</label>
                  <Badge variant={getTypeColor(selectedContract.type) as any}>
                    {selectedContract.type.replace('_', ' ').toUpperCase()}
                  </Badge>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Verification Status</label>
                  <Badge variant={selectedContract.verified ? 'success' : 'warning'}>
                    {selectedContract.verified ? 'Verified' : 'Unverified'}
                  </Badge>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Creator</label>
                  <div className="font-mono text-xs break-all">{selectedContract.creator}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Deployed At</label>
                  <div className="text-sm">{new Date(selectedContract.deployedAt).toLocaleString()}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Transaction Count</label>
                  <div className="text-lg">{selectedContract.transactions}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Balance</label>
                  <div className="text-lg font-bold">{selectedContract.balance}</div>
                </div>
                <Button className="w-full">View Transactions</Button>
                {selectedContract.verified && (
                  <Button className="w-full bg-green-600 hover:bg-green-700">View Source Code</Button>
                )}
                {!selectedContract.verified && (
                  <Button className="w-full bg-yellow-600 hover:bg-yellow-700">Verify Contract</Button>
                )}
              </div>
            ) : (
              <div className="text-center text-gray-500 py-8">
                Select a contract to view details
              </div>
            )}
          </Card>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
        <Card title="Total Contracts">
          <div className="text-center">
            <div className="text-3xl font-bold text-blue-600">{contracts.length}</div>
            <div className="text-sm text-gray-600">Deployed</div>
          </div>
        </Card>

        <Card title="Verified Contracts">
          <div className="text-center">
            <div className="text-3xl font-bold text-green-600">
              {contracts.filter(c => c.verified).length}
            </div>
            <div className="text-sm text-gray-600">Verified</div>
          </div>
        </Card>

        <Card title="Total Transactions">
          <div className="text-center">
            <div className="text-3xl font-bold text-purple-600">
              {contracts.reduce((sum, c) => sum + c.transactions, 0).toLocaleString()}
            </div>
            <div className="text-sm text-gray-600">Contract Calls</div>
          </div>
        </Card>

        <Card title="Total Value Locked">
          <div className="text-center">
            <div className="text-3xl font-bold text-orange-600">
              {contracts.reduce((sum, c) => sum + parseFloat(c.balance.replace(/[^\d.]/g, '')), 0).toLocaleString()} IPPAN
            </div>
            <div className="text-sm text-gray-600">TVL</div>
          </div>
        </Card>
      </div>
    </div>
  )
}
