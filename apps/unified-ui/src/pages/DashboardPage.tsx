import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Card, Badge, LoadingSpinner } from '../components/UI'
import { getConsensusStats, getMempoolStats, getNetworkStats, getNodeStatus } from '../lib/api'

function formatDuration(seconds: number) {
  if (!Number.isFinite(seconds) || seconds <= 0) {
    return '0s'
  }

  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = Math.floor(seconds % 60)

  if (hours > 0) {
    return `${hours}h ${minutes}m`
  }
  if (minutes > 0) {
    return `${minutes}m ${secs}s`
  }
  return `${secs}s`
}

export default function DashboardPage() {
  const statusQuery = useQuery({
    queryKey: ['node-status'],
    queryFn: getNodeStatus,
    refetchInterval: 15000,
  })

  const networkQuery = useQuery({
    queryKey: ['network-stats'],
    queryFn: getNetworkStats,
    refetchInterval: 30000,
  })

  const mempoolQuery = useQuery({
    queryKey: ['mempool-stats'],
    queryFn: getMempoolStats,
    refetchInterval: 15000,
  })

  const consensusQuery = useQuery({
    queryKey: ['consensus-stats'],
    queryFn: getConsensusStats,
    refetchInterval: 20000,
  })

  const uptime = useMemo(() => {
    if (!statusQuery.data?.node?.uptime_seconds) {
      return '—'
    }
    return formatDuration(statusQuery.data.node.uptime_seconds)
  }, [statusQuery.data?.node?.uptime_seconds])

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-slate-900">Network Overview</h1>
        <p className="text-sm text-slate-600">
          Data refreshes automatically every few seconds. Use the node selector to change the data source.
        </p>
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-3">
        <Card title="Node Status">
          {statusQuery.isLoading ? (
            <LoadingSpinner />
          ) : statusQuery.isError ? (
            <p className="text-sm text-red-600">Unable to reach the node status endpoint.</p>
          ) : statusQuery.data ? (
            <div className="space-y-3 text-sm text-slate-700">
              <div className="flex items-center justify-between">
                <span>Node ID</span>
                <span className="font-mono text-xs">{statusQuery.data.node.node_id}</span>
              </div>
              <div className="flex items-center justify-between">
                <span>Version</span>
                <span>{statusQuery.data.node.version}</span>
              </div>
              <div className="flex items-center justify-between">
                <span>Uptime</span>
                <span>{uptime}</span>
              </div>
              <div className="flex items-center justify-between">
                <span>Running</span>
                <Badge variant={statusQuery.data.node.is_running ? 'success' : 'error'}>
                  {statusQuery.data.node.is_running ? 'Online' : 'Offline'}
                </Badge>
              </div>
              <div className="flex items-center justify-between">
                <span>Current Block</span>
                <span className="font-semibold">{statusQuery.data.blockchain.current_height}</span>
              </div>
              <div className="flex items-center justify-between">
                <span>Total Transactions</span>
                <span className="font-semibold">{statusQuery.data.blockchain.total_transactions}</span>
              </div>
            </div>
          ) : null}
        </Card>

        <Card title="Network">
          {networkQuery.isLoading ? (
            <LoadingSpinner />
          ) : networkQuery.isError ? (
            <p className="text-sm text-red-600">Network information is unavailable.</p>
          ) : networkQuery.data ? (
            <div className="space-y-3 text-sm text-slate-700">
              <div className="flex items-center justify-between">
                <span>Protocol</span>
                <span>{networkQuery.data.protocol_version}</span>
              </div>
              <div className="flex items-center justify-between">
                <span>Network ID</span>
                <span className="font-mono text-xs">{networkQuery.data.network_id}</span>
              </div>
              <div className="flex items-center justify-between">
                <span>Connected Peers</span>
                <span className="font-semibold">{networkQuery.data.connected_peers}</span>
              </div>
              <div className="flex items-center justify-between">
                <span>Total Peers Seen</span>
                <span>{networkQuery.data.total_peers}</span>
              </div>
              <div className="flex items-center justify-between">
                <span>Uptime</span>
                <span>{formatDuration(networkQuery.data.uptime_seconds)}</span>
              </div>
            </div>
          ) : null}
        </Card>

        <Card title="Mempool">
          {mempoolQuery.isLoading ? (
            <LoadingSpinner />
          ) : mempoolQuery.isError ? (
            <p className="text-sm text-red-600">Mempool metrics are unavailable.</p>
          ) : mempoolQuery.data ? (
            <div className="space-y-3 text-sm text-slate-700">
              <div className="flex items-center justify-between">
                <span>Total Transactions</span>
                <span className="font-semibold">{mempoolQuery.data.total_transactions}</span>
              </div>
              <div className="flex items-center justify-between">
                <span>Unique Senders</span>
                <span>{mempoolQuery.data.total_senders}</span>
              </div>
              <div className="flex items-center justify-between">
                <span>Total Size</span>
                <span>{mempoolQuery.data.total_size.toLocaleString()} bytes</span>
              </div>
            </div>
          ) : null}
        </Card>
      </div>

      <Card title="Consensus">
        {consensusQuery.isLoading ? (
          <LoadingSpinner />
        ) : consensusQuery.isError ? (
          <p className="text-sm text-red-600">Consensus status is unavailable.</p>
        ) : consensusQuery.data ? (
          <div className="grid grid-cols-1 gap-4 text-sm text-slate-700 md:grid-cols-2 lg:grid-cols-4">
            <div className="flex flex-col gap-1">
              <span className="text-xs uppercase tracking-wide text-slate-500">Current Round</span>
              <span className="text-lg font-semibold">{consensusQuery.data.current_round}</span>
            </div>
            <div className="flex flex-col gap-1">
              <span className="text-xs uppercase tracking-wide text-slate-500">Validators</span>
              <span className="text-lg font-semibold">{consensusQuery.data.validators_count}</span>
            </div>
            <div className="flex flex-col gap-1">
              <span className="text-xs uppercase tracking-wide text-slate-500">Block Height</span>
              <span className="text-lg font-semibold">{consensusQuery.data.block_height}</span>
            </div>
            <div className="flex flex-col gap-1">
              <span className="text-xs uppercase tracking-wide text-slate-500">Status</span>
              <Badge variant={consensusQuery.data.consensus_status === 'healthy' ? 'success' : 'warning'}>
                {consensusQuery.data.consensus_status}
              </Badge>
            </div>
          </div>
        ) : null}
      </Card>
    </div>
  )
}
