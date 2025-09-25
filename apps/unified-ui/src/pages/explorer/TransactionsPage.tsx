import { useEffect, useMemo, useState } from 'react'
import { useSearchParams } from 'react-router-dom'
import { Card, Button, Badge, LoadingSpinner, Field, Input } from '../../components/UI'
import { getWalletTransactions } from '../../lib/walletApi'

interface Transaction {
  id: string
  from: string
  to: string
  amount: number
  nonce: number
  timestamp: number
  direction: 'send' | 'receive' | 'self' | 'other'
  hashtimer: string
}

const ITEMS_PER_PAGE = 20

const formatAmount = (value: number) => new Intl.NumberFormat(undefined, {
  minimumFractionDigits: 0,
  maximumFractionDigits: 6,
}).format(value)

const formatTimestamp = (value: number) => {
  if (!value) return '—'
  const millis = Math.floor(value / 1_000)
  return new Date(millis).toLocaleString()
}

const shortValue = (value: string) => {
  if (!value) return '—'
  return value.length <= 16 ? value : `${value.slice(0, 10)}…${value.slice(-6)}`
}

export default function TransactionsPage() {
  const [transactions, setTransactions] = useState<Transaction[]>([])
  const [selectedTx, setSelectedTx] = useState<Transaction | null>(null)
  const [searchTerm, setSearchTerm] = useState('')
  const [filterType, setFilterType] = useState<'all' | 'send' | 'receive' | 'self' | 'other'>('all')
  const [currentPage, setCurrentPage] = useState(1)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [lastAddress, setLastAddress] = useState<string | null>(null)
  const [searchParams, setSearchParams] = useSearchParams()
  const initialAddress = searchParams.get('address')

  const fetchTransactions = async (addressInput: string) => {
    const address = addressInput.trim()
    if (!address) {
      setError('Enter an IPPAN address to lookup transactions.')
      return
    }

    setIsLoading(true)
    setError(null)
    setSelectedTx(null)
    setTransactions([])

    try {
      const response = await getWalletTransactions(address)
      const parsed: Transaction[] = (response || []).map((tx: any) => ({
        id: tx.id ?? tx.tx_hash ?? '',
        from: tx.from ?? '',
        to: tx.to ?? '',
        amount: typeof tx.amount === 'string' ? Number(tx.amount) : tx.amount ?? 0,
        nonce: tx.nonce ?? 0,
        timestamp: typeof tx.timestamp === 'number'
          ? tx.timestamp
          : Number(tx.timestamp?.us ?? tx.timestamp?.[0] ?? 0),
        direction: (tx.direction ?? 'other') as Transaction['direction'],
        hashtimer: tx.hashtimer ?? '',
      }))

      setTransactions(parsed)
      setCurrentPage(1)
      setLastAddress(address)

      if (parsed.length === 0) {
        setError('No transactions found for this address yet.')
      }
    } catch (err: any) {
      console.error('Failed to fetch transactions', err)
      setError(err?.message || 'Unable to fetch transactions. Ensure the node RPC service is running.')
    } finally {
      setIsLoading(false)
    }
  }

  const handleSearch = () => {
    fetchTransactions(searchTerm)
    if (searchTerm.trim()) {
      setSearchParams({ address: searchTerm.trim() })
    }
  }

  useEffect(() => {
    if (initialAddress) {
      setSearchTerm(initialAddress)
      fetchTransactions(initialAddress)
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const filteredTransactions = useMemo(() => {
    return transactions.filter((tx) => {
      if (filterType === 'all') return true
      return tx.direction === filterType
    })
  }, [transactions, filterType])

  const totalPages = Math.max(1, Math.ceil(filteredTransactions.length / ITEMS_PER_PAGE))
  const pageStart = (currentPage - 1) * ITEMS_PER_PAGE
  const paginatedTransactions = filteredTransactions.slice(pageStart, pageStart + ITEMS_PER_PAGE)

  const directionLabel: Record<Transaction['direction'], string> = {
    send: 'Sent',
    receive: 'Received',
    self: 'Self Transfer',
    other: 'Other',
  }

  const directionVariant: Record<Transaction['direction'], 'success' | 'warning' | 'default'> = {
    send: 'warning',
    receive: 'success',
    self: 'default',
    other: 'default',
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Transactions</h1>
          <p className="text-sm text-gray-600">
            Look up confirmed transactions directly from the node RPC interface.
          </p>
        </div>
        <Badge variant="success">Live RPC</Badge>
      </div>

      <Card title="Lookup Transactions by Address">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <Field label="IPPAN Address">
            <Input
              placeholder="i0000…"
              value={searchTerm}
              onChange={(event) => setSearchTerm(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault()
                  handleSearch()
                }
              }}
            />
          </Field>

          <Field label="Direction Filter">
            <select
              value={filterType}
              onChange={(event) => {
                setFilterType(event.target.value as typeof filterType)
                setCurrentPage(1)
              }}
              className="w-full p-2 border border-gray-300 rounded-md"
            >
              <option value="all">All directions</option>
              <option value="receive">Inbound</option>
              <option value="send">Outbound</option>
              <option value="self">Self</option>
              <option value="other">Other</option>
            </select>
          </Field>

          <div className="flex items-end">
            <Button className="w-full" onClick={handleSearch} disabled={isLoading}>
              {isLoading ? 'Loading…' : 'Fetch Transactions'}
            </Button>
          </div>
        </div>

        <p className="text-xs text-gray-500 mt-3">
          The RPC server returns transactions for the provided address directly from storage.
          Results are limited to the records currently retained by the node.
        </p>

        {error && (
          <div className="mt-3 text-sm text-red-600 bg-red-50 border border-red-200 rounded-md p-3">
            {error}
          </div>
        )}
      </Card>

      <Card title={`Transactions ${lastAddress ? `for ${shortValue(lastAddress)}` : ''}`}>
        {isLoading ? (
          <div className="flex justify-center items-center h-48">
            <LoadingSpinner />
          </div>
        ) : paginatedTransactions.length === 0 ? (
          <div className="text-center text-gray-500 py-12">
            {lastAddress
              ? 'No transactions available for this address yet.'
              : 'Enter an address above to load transactions.'}
          </div>
        ) : (
          <div className="space-y-3">
            {paginatedTransactions.map((tx) => (
              <button
                key={tx.id}
                type="button"
                onClick={() => setSelectedTx(tx)}
                className={`w-full text-left border rounded-lg p-4 transition-all ${
                  selectedTx?.id === tx.id ? 'border-blue-500 bg-blue-50' : 'border-gray-200 hover:border-blue-400'
                }`}
              >
                <div className="flex flex-wrap items-center gap-2 mb-2">
                  <Badge variant={directionVariant[tx.direction]}>{directionLabel[tx.direction]}</Badge>
                  <span className="text-sm text-gray-600">Nonce {tx.nonce}</span>
                  <span className="text-sm text-gray-600">{formatTimestamp(tx.timestamp)}</span>
                </div>
                <div className="text-sm text-gray-700 space-y-1">
                  <div><strong>Hash:</strong> {shortValue(tx.id)}</div>
                  <div><strong>From:</strong> {shortValue(tx.from)}</div>
                  <div><strong>To:</strong> {shortValue(tx.to)}</div>
                  <div><strong>Amount:</strong> {formatAmount(tx.amount)} IPN</div>
                </div>
              </button>
            ))}
          </div>
        )}

        {paginatedTransactions.length > 0 && (
          <div className="flex justify-between items-center mt-4 text-sm text-gray-600">
            <span>
              Showing {pageStart + 1}–{Math.min(pageStart + ITEMS_PER_PAGE, filteredTransactions.length)} of
              {' '}
              {filteredTransactions.length}
            </span>
            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                disabled={currentPage === 1}
                onClick={() => setCurrentPage((page) => Math.max(1, page - 1))}
              >
                Previous
              </Button>
              <span>Page {currentPage} of {totalPages}</span>
              <Button
                variant="ghost"
                disabled={currentPage === totalPages}
                onClick={() => setCurrentPage((page) => Math.min(totalPages, page + 1))}
              >
                Next
              </Button>
            </div>
          </div>
        )}
      </Card>

      {selectedTx && (
        <Card title="Transaction Details">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm text-gray-700">
            <div>
              <h3 className="font-semibold text-gray-900 mb-2">Identifiers</h3>
              <p><strong>Transaction Hash:</strong> {selectedTx.id}</p>
              <p><strong>HashTimer:</strong> {selectedTx.hashtimer || '—'}</p>
              <p><strong>Timestamp:</strong> {formatTimestamp(selectedTx.timestamp)}</p>
            </div>
            <div>
              <h3 className="font-semibold text-gray-900 mb-2">Participants</h3>
              <p><strong>From:</strong> {selectedTx.from}</p>
              <p><strong>To:</strong> {selectedTx.to}</p>
              <p><strong>Direction:</strong> {directionLabel[selectedTx.direction]}</p>
              <p><strong>Amount:</strong> {formatAmount(selectedTx.amount)} IPN</p>
            </div>
          </div>
        </Card>
      )}
    </div>
  )
}
