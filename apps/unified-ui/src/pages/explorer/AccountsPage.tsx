import { useEffect, useMemo, useState } from 'react'
import { Card, Button, Badge, LoadingSpinner, Field, Input, Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from '../../components/UI'
import { AccountSummary, getAccounts } from '../../lib/api'

interface AccountRow extends AccountSummary {
  transactionCount?: number
}

const formatAmount = (value: number) => new Intl.NumberFormat(undefined, {
  minimumFractionDigits: 0,
  maximumFractionDigits: 6,
}).format(value)

export default function AccountsPage() {
  const [accounts, setAccounts] = useState<AccountRow[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [searchTerm, setSearchTerm] = useState('')
  const [sortBy, setSortBy] = useState<'balance_desc' | 'balance_asc' | 'nonce_desc' | 'nonce_asc' | 'address'>('balance_desc')

  useEffect(() => {
    const fetchAccounts = async () => {
      setIsLoading(true)
      setError(null)

      try {
        const list = await getAccounts()
        setAccounts(list)
      } catch (err: any) {
        console.error('Failed to load accounts', err)
        setError(err?.message || 'Unable to load accounts from the RPC service.')
      } finally {
        setIsLoading(false)
      }
    }

    fetchAccounts()
  }, [])

  const filteredAccounts = useMemo(() => {
    const needle = searchTerm.trim().toLowerCase()

    const filtered = needle
      ? accounts.filter((account) => account.address.toLowerCase().includes(needle))
      : accounts

    const sorted = [...filtered]

    sorted.sort((a, b) => {
      switch (sortBy) {
        case 'balance_desc':
          return b.balance - a.balance
        case 'balance_asc':
          return a.balance - b.balance
        case 'nonce_desc':
          return b.nonce - a.nonce
        case 'nonce_asc':
          return a.nonce - b.nonce
        case 'address':
          return a.address.localeCompare(b.address)
        default:
          return 0
      }
    })

    return sorted
  }, [accounts, searchTerm, sortBy])

  const totalSupply = accounts.reduce((acc, account) => acc + account.balance, 0)

  if (isLoading) {
    return (
      <div className="flex justify-center items-center h-64">
        <LoadingSpinner />
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Accounts</h1>
          <p className="text-sm text-gray-600">All accounts tracked by this node&apos;s local storage.</p>
        </div>
        <Badge variant="success">Live RPC</Badge>
      </div>

      <Card title="Search & Sort">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Field label="Account Address">
            <Input
              placeholder="Search by address…"
              value={searchTerm}
              onChange={(event) => setSearchTerm(event.target.value)}
            />
          </Field>

          <Field label="Sort By">
            <Select value={sortBy} onValueChange={(value) => setSortBy(value as typeof sortBy)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="balance_desc">Balance (high → low)</SelectItem>
                <SelectItem value="balance_asc">Balance (low → high)</SelectItem>
                <SelectItem value="nonce_desc">Nonce (high → low)</SelectItem>
                <SelectItem value="nonce_asc">Nonce (low → high)</SelectItem>
                <SelectItem value="address">Address</SelectItem>
              </SelectContent>
            </Select>
          </Field>
        </div>

        {error && (
          <div className="mt-3 text-sm text-red-600 bg-red-50 border border-red-200 rounded-md p-3">
            {error}
          </div>
        )}
      </Card>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <Card title="Summary" className="lg:col-span-1">
          <div className="space-y-3 text-sm text-gray-700">
            <p><strong>Total Accounts:</strong> {accounts.length}</p>
            <p><strong>Total Balance:</strong> {formatAmount(totalSupply)} IPN</p>
            <p><strong>Average Balance:</strong> {accounts.length ? formatAmount(totalSupply / accounts.length) : '0'} IPN</p>
          </div>
          <Button className="w-full mt-4" onClick={() => window.location.reload()}>Refresh</Button>
        </Card>

        <Card title={`Accounts (${filteredAccounts.length})`} className="lg:col-span-2">
          {filteredAccounts.length === 0 ? (
            <div className="text-center text-gray-500 py-12">
              {searchTerm ? 'No accounts match your search.' : 'No accounts found on this node yet.'}
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="min-w-full text-sm text-left">
                <thead>
                  <tr className="text-gray-500 border-b">
                    <th className="py-2 pr-4">Address</th>
                    <th className="py-2 pr-4">Balance (IPN)</th>
                    <th className="py-2 pr-4">Nonce</th>
                    <th className="py-2 pr-4">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {filteredAccounts.map((account) => (
                    <tr key={account.address} className="border-b last:border-0">
                      <td className="py-3 pr-4 font-mono text-xs text-gray-700 break-all">{account.address}</td>
                      <td className="py-3 pr-4">{formatAmount(account.balance)}</td>
                      <td className="py-3 pr-4">{account.nonce}</td>
                      <td className="py-3 pr-4">
                        <div className="flex flex-wrap gap-2">
                          <Button
                            variant="ghost"
                            onClick={() => navigator.clipboard?.writeText(account.address)}
                          >
                            Copy
                          </Button>
                          <Button
                            variant="ghost"
                            onClick={() => window.open(`/explorer/transactions?address=${account.address}`, '_blank')}
                          >
                            View Transactions
                          </Button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Card>
      </div>
    </div>
  )
}
