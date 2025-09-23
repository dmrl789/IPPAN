import { useState, useEffect } from 'react'
import { Card, Button, Badge, LoadingSpinner, Field, Input, Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from '../../components/UI'

interface Account {
  address: string
  balance: string
  nonce: number
  transactionCount: number
  firstSeen: string
  lastSeen: string
  type: 'user' | 'validator'
  stakedAmount?: string
  validatorStatus?: 'active' | 'inactive'
}

export default function AccountsPage() {
  const [accounts, setAccounts] = useState<Account[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [selectedAccount, setSelectedAccount] = useState<Account | null>(null)
  const [searchTerm, setSearchTerm] = useState('')
  const [filterType, setFilterType] = useState<string>('all')
  const [sortBy, setSortBy] = useState<string>('balance')
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc')
  const [isSearching, setIsSearching] = useState(false)

  useEffect(() => {
    const mockAccounts: Account[] = [
      {
        address: "i0000000000000000000000000000000000000000000000000000000000000000",
        balance: "1000.0",
        nonce: 42,
        transactionCount: 156,
        firstSeen: '2024-01-15T10:30:00Z',
        lastSeen: new Date().toISOString(),
        type: 'user'
      },
      {
        address: "i1111111111111111111111111111111111111111111111111111111111111111",
        balance: "2500.0",
        nonce: 23,
        transactionCount: 89,
        firstSeen: '2024-02-20T14:15:00Z',
        lastSeen: new Date(Date.now() - 3600000).toISOString(),
        type: 'validator',
        stakedAmount: '10,000.00 IPPAN',
        validatorStatus: 'active'
      },
      {
        address: "i2222222222222222222222222222222222222222222222222222222222222222",
        balance: "5000.0",
        nonce: 67,
        transactionCount: 312,
        firstSeen: '2024-01-05T08:20:00Z',
        lastSeen: new Date(Date.now() - 7200000).toISOString(),
        type: 'validator',
        stakedAmount: '25,000.00 IPPAN',
        validatorStatus: 'active'
      },
      {
        address: "i3333333333333333333333333333333333333333333333333333333333333333",
        balance: "750.5",
        nonce: 12,
        transactionCount: 45,
        firstSeen: '2024-03-15T16:45:00Z',
        lastSeen: new Date(Date.now() - 1800000).toISOString(),
        type: 'user'
      },
      {
        address: "i4444444444444444444444444444444444444444444444444444444444444444",
        balance: "15000.0",
        nonce: 34,
        transactionCount: 189,
        firstSeen: '2024-01-20T11:30:00Z',
        lastSeen: new Date(Date.now() - 5400000).toISOString(),
        type: 'validator',
        stakedAmount: '50,000.00 IPPAN',
        validatorStatus: 'inactive'
      },
      {
        address: "i5555555555555555555555555555555555555555555555555555555555555555",
        balance: "250.0",
        nonce: 8,
        transactionCount: 23,
        firstSeen: '2024-03-20T14:15:00Z',
        lastSeen: new Date(Date.now() - 3600000).toISOString(),
        type: 'user'
      },
      {
        address: "i6666666666666666666666666666666666666666666666666666666666666666",
        balance: "3200.0",
        nonce: 156,
        transactionCount: 445,
        firstSeen: '2024-01-10T09:30:00Z',
        lastSeen: new Date(Date.now() - 7200000).toISOString(),
        type: 'user'
      },
      {
        address: "i7777777777777777777777777777777777777777777777777777777777777777",
        balance: "850.0",
        nonce: 29,
        transactionCount: 78,
        firstSeen: '2024-03-05T13:20:00Z',
        lastSeen: new Date(Date.now() - 1800000).toISOString(),
        type: 'user'
      }
    ]

    setAccounts(mockAccounts)
    setIsLoading(false)
  }, [])

  const handleSearch = async () => {
    setIsSearching(true)
    // Simulate API call delay
    await new Promise(resolve => setTimeout(resolve, 500))
    setIsSearching(false)
  }

  const filteredAndSortedAccounts = accounts
    .filter(account => {
      const matchesSearch = account.address.toLowerCase().includes(searchTerm.toLowerCase())
      const matchesType = filterType === 'all' || account.type === filterType
      return matchesSearch && matchesType
    })
    .sort((a, b) => {
      let comparison = 0
      
      switch (sortBy) {
        case 'balance':
          comparison = parseFloat(a.balance) - parseFloat(b.balance)
          break
        case 'transactionCount':
          comparison = a.transactionCount - b.transactionCount
          break
        case 'nonce':
          comparison = a.nonce - b.nonce
          break
        case 'lastSeen':
          comparison = new Date(a.lastSeen).getTime() - new Date(b.lastSeen).getTime()
          break
        default:
          comparison = 0
      }
      
      return sortOrder === 'asc' ? comparison : -comparison
    })

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
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Accounts</h1>
          <p className="text-sm text-gray-600">
            L1 IPPAN accounts (Users & Validators) • Smart contracts run on L2
          </p>
        </div>
        <Badge variant="success">Live Network</Badge>
      </div>

      <Card title="🔍 Search & Filter Accounts">
        <div className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            <div className="lg:col-span-2">
              <Field label="Account Address">
                <div className="flex gap-2">
                  <Input
                    placeholder="Search by address..."
                    value={searchTerm}
                    onChange={(e) => setSearchTerm(e.target.value)}
                    onKeyPress={(e) => e.key === 'Enter' && handleSearch()}
                    className="flex-1"
                  />
                  <Button 
                    onClick={handleSearch}
                    disabled={isSearching}
                    className="px-4"
                  >
                    {isSearching ? '⏳' : '🔍'} Search
                  </Button>
                </div>
              </Field>
            </div>
            
            <div>
              <Field label="Account Type">
                <Select value={filterType} onValueChange={setFilterType}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Types</SelectItem>
                    <SelectItem value="user">Users</SelectItem>
                    <SelectItem value="validator">Validators</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
            </div>
            
            <div>
              <Field label="Sort By">
                <Select value={sortBy} onValueChange={setSortBy}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="balance">Balance</SelectItem>
                    <SelectItem value="transactionCount">Transactions</SelectItem>
                    <SelectItem value="nonce">Nonce</SelectItem>
                    <SelectItem value="lastSeen">Last Seen</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
            </div>
          </div>
          
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-2">
                <input
                  type="radio"
                  id="sort-asc"
                  name="sortOrder"
                  value="asc"
                  checked={sortOrder === 'asc'}
                  onChange={(e) => setSortOrder(e.target.value as 'asc' | 'desc')}
                  className="w-4 h-4"
                />
                <label htmlFor="sort-asc" className="text-sm">Ascending</label>
              </div>
              <div className="flex items-center gap-2">
                <input
                  type="radio"
                  id="sort-desc"
                  name="sortOrder"
                  value="desc"
                  checked={sortOrder === 'desc'}
                  onChange={(e) => setSortOrder(e.target.value as 'asc' | 'desc')}
                  className="w-4 h-4"
                />
                <label htmlFor="sort-desc" className="text-sm">Descending</label>
              </div>
            </div>
            
            <div className="text-sm text-gray-600">
              Showing {filteredAndSortedAccounts.length} of {accounts.length} accounts
            </div>
          </div>
        </div>
      </Card>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2">
          <Card title={`📊 Accounts (${filteredAndSortedAccounts.length})`}>
            <div className="space-y-3">
              {filteredAndSortedAccounts.length === 0 ? (
                <div className="text-center text-gray-500 py-8">
                  <div className="text-4xl mb-2">🔍</div>
                  <div>No accounts found</div>
                  <div className="text-sm">Try adjusting your search criteria</div>
                </div>
              ) : (
                filteredAndSortedAccounts.map((account) => (
                <div
                  key={account.address}
                  className={`p-4 border rounded-lg cursor-pointer transition-all hover:shadow-md ${
                    selectedAccount?.address === account.address ? 'border-blue-500 bg-blue-50' : 'border-gray-200'
                  }`}
                  onClick={() => setSelectedAccount(account)}
                >
                  <div className="flex justify-between items-start">
                    <div className="flex-1">
                      <div className="flex items-center space-x-2 mb-2">
                        <Badge variant={account.type === 'validator' ? 'success' : 'blue'}>
                          {account.type.toUpperCase()}
                        </Badge>
                        {account.type === 'validator' && (
                          <Badge variant={account.validatorStatus === 'active' ? 'success' : 'warning'}>
                            {account.validatorStatus}
                          </Badge>
                        )}
                      </div>
                      <div className="text-sm text-gray-600 space-y-1">
                        <div>Address: {account.address.substring(0, 16)}...</div>
                        <div>Balance: {account.balance}</div>
                        <div>Transactions: {account.transactionCount}</div>
                        {account.stakedAmount && <div>Staked: {account.stakedAmount}</div>}
                      </div>
                    </div>
                    <div className="text-right text-sm text-gray-600">
                      <div>{account.balance}</div>
                      <div>Nonce: {account.nonce}</div>
                      <div>{new Date(account.lastSeen).toLocaleDateString()}</div>
                    </div>
                  </div>
                </div>
                ))
              )}
            </div>
          </Card>
        </div>

        <div className="lg:col-span-1">
          <Card title="📋 Account Details">
            {selectedAccount ? (
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700">Address</label>
                  <div className="font-mono text-xs break-all bg-gray-100 p-2 rounded">
                    {selectedAccount.address}
                  </div>
                  <Button 
                    onClick={() => navigator.clipboard?.writeText(selectedAccount.address)}
                    className="mt-2 text-xs px-2 py-1"
                  >
                    📋 Copy Address
                  </Button>
                </div>
                
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700">Balance</label>
                    <div className="text-lg font-bold text-green-600">{selectedAccount.balance} IPPAN</div>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700">Type</label>
                    <Badge variant={selectedAccount.type === 'validator' ? 'success' : 'blue'}>
                      {selectedAccount.type.toUpperCase()}
                    </Badge>
                  </div>
                </div>

                {selectedAccount.stakedAmount && (
                  <div>
                    <label className="block text-sm font-medium text-gray-700">Staked Amount</label>
                    <div className="text-lg font-semibold text-blue-600">{selectedAccount.stakedAmount}</div>
                  </div>
                )}
                
                {selectedAccount.validatorStatus && (
                  <div>
                    <label className="block text-sm font-medium text-gray-700">Validator Status</label>
                    <Badge variant={selectedAccount.validatorStatus === 'active' ? 'success' : 'warning'}>
                      {selectedAccount.validatorStatus.toUpperCase()}
                    </Badge>
                  </div>
                )}
                
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700">Nonce</label>
                    <div className="text-lg">{selectedAccount.nonce}</div>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700">Transactions</label>
                    <div className="text-lg">{selectedAccount.transactionCount}</div>
                  </div>
                </div>
                
                <div>
                  <label className="block text-sm font-medium text-gray-700">First Seen</label>
                  <div className="text-sm text-gray-600">{new Date(selectedAccount.firstSeen).toLocaleString()}</div>
                </div>
                
                <div>
                  <label className="block text-sm font-medium text-gray-700">Last Seen</label>
                  <div className="text-sm text-gray-600">{new Date(selectedAccount.lastSeen).toLocaleString()}</div>
                </div>
                
                <div className="pt-4 border-t">
                  <Button 
                    className="w-full mb-2"
                    onClick={() => window.open(`/explorer/transactions?address=${selectedAccount.address}`, '_blank')}
                  >
                    📊 View Transactions
                  </Button>
                  <Button 
                    className="w-full bg-gray-600 hover:bg-gray-700"
                    onClick={() => setSelectedAccount(null)}
                  >
                    ✕ Clear Selection
                  </Button>
                </div>
              </div>
            ) : (
              <div className="text-center text-gray-500 py-8">
                <div className="text-4xl mb-2">👆</div>
                <div>Select an account to view details</div>
                <div className="text-sm">Click on any account from the list</div>
              </div>
            )}
          </Card>
        </div>
      </div>
    </div>
  )
}
