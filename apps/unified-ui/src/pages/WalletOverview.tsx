import { useEffect, useMemo, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Card, Button, Field, Input, Badge, LoadingSpinner } from '../components/UI'
import {
  getWalletBalance,
  getWalletTransactions,
  validateAddress,
  type WalletBalanceResponse,
  type WalletTransaction,
} from '../lib/walletApi'

const STORAGE_KEY = 'ippan.wallet.address'
const ADDRESS_REGEX = /^i[0-9a-fA-F]{64}$/
const ATOMIC_MULTIPLIER = 1_000_000_000

type ConnectionState = 'idle' | 'connecting' | 'error'

declare global {
  interface Window {
    ippan?: {
      connect?: () => Promise<{ address: string } | string>
      getAddress?: () => Promise<string>
    }
  }
}

function formatIpn(value?: number) {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return '0 IPN'
  }

  const ipn = value / ATOMIC_MULTIPLIER
  return `${ipn.toLocaleString(undefined, {
    maximumFractionDigits: 4,
  })} IPN`
}

function formatAtomic(value?: number) {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return '0'
  }
  return value.toLocaleString()
}

function formatTimestamp(timestamp?: number) {
  if (!timestamp) {
    return '—'
  }

  const date = new Date(Math.floor(timestamp / 1000))
  if (Number.isNaN(date.getTime())) {
    return '—'
  }

  return `${date.toLocaleDateString()} ${date.toLocaleTimeString()}`
}

function truncateAddress(address: string, chars = 10) {
  if (!address) return ''
  return `${address.slice(0, chars)}…${address.slice(-chars)}`
}

export default function WalletOverview() {
  const queryClient = useQueryClient()
  const [address, setAddress] = useState<string>(() => {
    if (typeof window === 'undefined') return ''
    return window.localStorage.getItem(STORAGE_KEY) || ''
  })
  const [formValue, setFormValue] = useState(address)
  const [connectionState, setConnectionState] = useState<ConnectionState>('idle')
  const [connectionMessage, setConnectionMessage] = useState<string>('')
  const [warning, setWarning] = useState<string>('')
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    setFormValue(address)
  }, [address])

  useEffect(() => {
    if (!copied) return
    const timeout = window.setTimeout(() => setCopied(false), 1500)
    return () => window.clearTimeout(timeout)
  }, [copied])

  const { data: balance, isLoading: balanceLoading, isError: balanceError, refetch: refetchBalance } = useQuery({
    queryKey: ['wallet', 'balance', address],
    queryFn: () => getWalletBalance(address),
    enabled: ADDRESS_REGEX.test(address),
    refetchInterval: 20000,
  })

  const {
    data: transactions,
    isLoading: transactionsLoading,
    isError: transactionsError,
    refetch: refetchTransactions,
  } = useQuery({
    queryKey: ['wallet', 'transactions', address],
    queryFn: () => getWalletTransactions(address),
    enabled: ADDRESS_REGEX.test(address),
    refetchInterval: 25000,
  })

  const latestTransactions = useMemo<WalletTransaction[]>(() => {
    if (!transactions || !transactions.length) {
      return []
    }

    return [...transactions].sort((a, b) => (b.timestamp ?? 0) - (a.timestamp ?? 0)).slice(0, 10)
  }, [transactions])

  useEffect(() => {
    if (!address) {
      queryClient.removeQueries({ queryKey: ['wallet'] })
    }
  }, [address, queryClient])

  const handleDisconnect = () => {
    setAddress('')
    setFormValue('')
    setConnectionMessage('')
    setWarning('')
    if (typeof window !== 'undefined') {
      window.localStorage.removeItem(STORAGE_KEY)
    }
  }

  const saveAddress = (value: string) => {
    setAddress(value)
    if (typeof window !== 'undefined') {
      window.localStorage.setItem(STORAGE_KEY, value)
    }
  }

  const handleConnect = async () => {
    const candidate = formValue.trim()

    if (!candidate) {
      setConnectionState('error')
      setConnectionMessage('Enter an IPPAN address to continue.')
      return
    }

    if (!ADDRESS_REGEX.test(candidate)) {
      setConnectionState('error')
      setConnectionMessage('Addresses must start with "i" followed by 64 hexadecimal characters.')
      return
    }

    setConnectionState('connecting')
    setConnectionMessage('Validating address…')
    setWarning('')

    try {
      const valid = await validateAddress(candidate)
      if (!valid) {
        setConnectionState('error')
        setConnectionMessage('The node rejected this address. Please double-check and try again.')
        return
      }
    } catch (error) {
      console.warn('Address validation failed:', error)
      setWarning('Connected without remote validation. The validation endpoint is unavailable.')
    }

    saveAddress(candidate)
    setConnectionState('idle')
    setConnectionMessage('Wallet connected successfully.')
    refetchBalance()
    refetchTransactions()
  }

  const connectWithProvider = async () => {
    if (!window.ippan?.connect) {
      setConnectionState('error')
      setConnectionMessage('No IPPAN wallet provider was detected in the browser.')
      return
    }

    setConnectionState('connecting')
    setConnectionMessage('Requesting wallet permission…')
    setWarning('')

    try {
      const result = await window.ippan.connect()
      const walletAddress = typeof result === 'string' ? result : result?.address

      if (!walletAddress) {
        throw new Error('Wallet did not return an address.')
      }

      if (!ADDRESS_REGEX.test(walletAddress)) {
        throw new Error('The connected wallet returned an unexpected address format.')
      }

      saveAddress(walletAddress)
      setFormValue(walletAddress)
      setConnectionState('idle')
      setConnectionMessage('Wallet connected successfully.')
      refetchBalance()
      refetchTransactions()
    } catch (error) {
      console.error('Wallet connection failed:', error)
      setConnectionState('error')
      setConnectionMessage(error instanceof Error ? error.message : 'Failed to connect to the wallet.')
    }
  }

  const handleCopy = async () => {
    if (!address) return
    try {
      await navigator.clipboard.writeText(address)
      setCopied(true)
    } catch (error) {
      console.warn('Clipboard copy failed:', error)
    }
  }

  const walletConnected = ADDRESS_REGEX.test(address)
  const pending = (balance as WalletBalanceResponse | undefined)?.pending_transactions ?? []

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-slate-900">Wallet Overview</h1>
        {walletConnected && (
          <div className="flex items-center gap-2">
            <Button onClick={() => { refetchBalance(); refetchTransactions() }}>
              Refresh
            </Button>
            <Button variant="secondary" onClick={handleDisconnect}>
              Disconnect
            </Button>
          </div>
        )}
      </div>

      <Card title="Connection">
        <div className="space-y-4 text-sm">
          <p className="text-slate-600">
            Connect a wallet address to monitor balances and on-chain activity. Addresses are stored locally in your browser only.
          </p>
          <Field label="Wallet address">
            <Input
              placeholder="i0000…"
              value={formValue}
              onChange={(event) => setFormValue(event.target.value)}
              spellCheck={false}
            />
          </Field>
          <div className="flex flex-wrap gap-3">
            <Button onClick={handleConnect} disabled={connectionState === 'connecting'}>
              {connectionState === 'connecting' ? 'Connecting…' : 'Save address'}
            </Button>
            <Button
              type="button"
              variant="secondary"
              onClick={connectWithProvider}
              disabled={connectionState === 'connecting'}
            >
              Detect browser wallet
            </Button>
          </div>
          {connectionMessage && (
            <p
              className={`text-sm ${
                connectionState === 'error' ? 'text-red-600' : connectionState === 'connecting' ? 'text-slate-600' : 'text-emerald-600'
              }`}
            >
              {connectionMessage}
            </p>
          )}
          {warning && <p className="text-sm text-amber-600">{warning}</p>}
        </div>
      </Card>

      {walletConnected && (
        <>
          <div className="grid grid-cols-1 gap-6 md:grid-cols-2 xl:grid-cols-3">
            <Card title="Balances">
              {balanceLoading ? (
                <LoadingSpinner />
              ) : balanceError ? (
                <p className="text-sm text-red-600">Unable to fetch wallet balance from the node.</p>
              ) : balance ? (
                <div className="space-y-3 text-sm text-slate-700">
                  <div className="flex items-center justify-between">
                    <span>Available</span>
                    <div className="text-right">
                      <div className="font-semibold">{formatIpn(balance.balance)}</div>
                      <div className="text-xs text-slate-500">{formatAtomic(balance.balance)} atomic units</div>
                    </div>
                  </div>
                  <div className="flex items-center justify-between">
                    <span>Staked</span>
                    <div className="text-right">
                      <div className="font-semibold">{formatIpn(balance.staked)}</div>
                      <div className="text-xs text-slate-500">{formatAtomic(balance.staked)} atomic units</div>
                    </div>
                  </div>
                  <div className="flex items-center justify-between">
                    <span>Rewards</span>
                    <div className="text-right">
                      <div className="font-semibold">{formatIpn(balance.rewards)}</div>
                      <div className="text-xs text-slate-500">{formatAtomic(balance.rewards)} atomic units</div>
                    </div>
                  </div>
                  <div className="flex items-center justify-between border-t pt-3">
                    <span>Total</span>
                    <div className="text-right font-semibold">
                      {formatIpn((balance.balance ?? 0) + (balance.staked ?? 0) + (balance.rewards ?? 0))}
                    </div>
                  </div>
                </div>
              ) : null}
            </Card>

            <Card title="Wallet details">
              <div className="space-y-3 text-sm text-slate-700">
                <div className="flex items-center justify-between gap-3">
                  <span>Address</span>
                  <div className="flex items-center gap-2">
                    <code className="rounded bg-slate-100 px-2 py-1 text-xs text-slate-600">{truncateAddress(address)}</code>
                    <Button variant="secondary" onClick={handleCopy}>
                      {copied ? 'Copied' : 'Copy'}
                    </Button>
                  </div>
                </div>
                <div className="flex items-center justify-between">
                  <span>Nonce</span>
                  <span className="font-semibold">{balance?.nonce ?? 0}</span>
                </div>
                <div className="flex items-start justify-between gap-3">
                  <span>Pending transactions</span>
                  <div className="space-y-1 text-right">
                    <Badge variant={pending.length ? 'warning' : 'success'}>
                      {pending.length ? `${pending.length} pending` : 'None'}
                    </Badge>
                    {pending.slice(0, 3).map((tx) => (
                      <div key={tx} className="font-mono text-[11px] text-slate-500">
                        {truncateAddress(tx, 6)}
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </Card>

            <Card title="Node sync">
              <div className="space-y-3 text-sm text-slate-700">
                <p className="text-slate-600">
                  Balances are fetched directly from the connected IPPAN node. If numbers look stale, refresh or verify the node
                  status on the dashboard.
                </p>
                <Button variant="secondary" onClick={() => { refetchBalance(); refetchTransactions() }}>
                  Refresh data now
                </Button>
              </div>
            </Card>
          </div>

          <Card title="Recent transactions">
            {transactionsLoading ? (
              <LoadingSpinner />
            ) : transactionsError ? (
              <p className="text-sm text-red-600">Unable to fetch transactions from the node.</p>
            ) : latestTransactions.length === 0 ? (
              <p className="text-sm text-slate-600">No transactions were found for this wallet yet.</p>
            ) : (
              <div className="overflow-x-auto">
                <table className="min-w-full divide-y divide-slate-200 text-sm">
                  <thead className="bg-slate-50 text-left text-xs font-semibold uppercase tracking-wide text-slate-500">
                    <tr>
                      <th className="px-3 py-2">Type</th>
                      <th className="px-3 py-2">Amount</th>
                      <th className="px-3 py-2">Counterparty</th>
                      <th className="px-3 py-2">Nonce</th>
                      <th className="px-3 py-2">HashTimer</th>
                      <th className="px-3 py-2">Timestamp</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-200 bg-white">
                    {latestTransactions.map((tx) => {
                      const direction = tx.direction || (tx.from === address ? 'send' : tx.to === address ? 'receive' : 'other')
                      const counterparty = direction === 'send' ? tx.to : tx.from
                      return (
                        <tr key={tx.id}>
                          <td className="px-3 py-2 font-medium capitalize text-slate-700">
                            {direction}
                          </td>
                          <td className="px-3 py-2">
                            <div className="text-slate-800">{formatIpn(tx.amount)}</div>
                            <div className="text-xs text-slate-500">{formatAtomic(tx.amount)}</div>
                          </td>
                          <td className="px-3 py-2 font-mono text-xs text-slate-600">{truncateAddress(counterparty)}</td>
                          <td className="px-3 py-2 text-slate-700">{tx.nonce}</td>
                          <td className="px-3 py-2 font-mono text-[11px] text-slate-500">{truncateAddress(tx.hashtimer ?? '', 6)}</td>
                          <td className="px-3 py-2 text-slate-600">{formatTimestamp(tx.timestamp)}</td>
                        </tr>
                      )
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </Card>
        </>
      )}
    </div>
  )
}
