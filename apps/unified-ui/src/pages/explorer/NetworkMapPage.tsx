import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Card, Badge, LoadingSpinner } from '../../components/UI'
import { getNetworkStats, getNodeStatus, getP2PPeers } from '../../lib/api'

function formatDuration(seconds: number): string {
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

export default function NetworkMapPage() {
  const networkQuery = useQuery({
    queryKey: ['network-stats', 'map-view'],
    queryFn: getNetworkStats,
    refetchInterval: 15000,
  })

  const nodeStatusQuery = useQuery({
    queryKey: ['node-status', 'map-view'],
    queryFn: getNodeStatus,
    refetchInterval: 15000,
  })

  const peersQuery = useQuery({
    queryKey: ['p2p-peers'],
    queryFn: getP2PPeers,
    refetchInterval: 10000,
  })

  const peerList = peersQuery.data ?? []
  const onlinePeers = useMemo(() => peerList.length, [peerList])

  return (
    <div className="space-y-6">
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold text-slate-900">Network Topology</h1>
          <p className="text-sm text-slate-600">
            Live peer data is sourced directly from the node's P2P subsystem. Use the gateway configuration to control which
            peers are exposed.
          </p>
        </div>
        <Badge variant="success">Live RPC</Badge>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <Card title="Network Status" className="lg:col-span-2">
          {networkQuery.isLoading || nodeStatusQuery.isLoading ? (
            <div className="flex items-center justify-center h-48">
              <LoadingSpinner />
            </div>
          ) : networkQuery.isError || nodeStatusQuery.isError ? (
            <div className="text-sm text-red-600">
              Unable to load network telemetry. Confirm the node RPC service is running and reachable.
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6 text-sm text-slate-700">
              <div>
                <div className="text-xs uppercase tracking-wide text-slate-500">Network ID</div>
                <div className="font-mono text-xs">{networkQuery.data?.network_id}</div>
              </div>
              <div>
                <div className="text-xs uppercase tracking-wide text-slate-500">Protocol Version</div>
                <div>{networkQuery.data?.protocol_version}</div>
              </div>
              <div>
                <div className="text-xs uppercase tracking-wide text-slate-500">Connected Peers</div>
                <div className="text-lg font-semibold">{networkQuery.data?.connected_peers ?? 0}</div>
              </div>
              <div>
                <div className="text-xs uppercase tracking-wide text-slate-500">Total Peers Seen</div>
                <div className="text-lg font-semibold">{networkQuery.data?.total_peers ?? 0}</div>
              </div>
              <div>
                <div className="text-xs uppercase tracking-wide text-slate-500">Node Uptime</div>
                <div>{formatDuration(nodeStatusQuery.data?.node?.uptime_seconds ?? 0)}</div>
              </div>
              <div>
                <div className="text-xs uppercase tracking-wide text-slate-500">Latest Block Height</div>
                <div>{nodeStatusQuery.data?.blockchain?.current_height?.toLocaleString() ?? '—'}</div>
              </div>
            </div>
          )}
        </Card>

        <Card title="Peer Summary">
          {peersQuery.isLoading ? (
            <div className="flex items-center justify-center h-48">
              <LoadingSpinner />
            </div>
          ) : peersQuery.isError ? (
            <div className="text-sm text-red-600">Unable to retrieve the peer list.</div>
          ) : peerList.length === 0 ? (
            <div className="text-sm text-slate-600">No peers are currently connected.</div>
          ) : (
            <div className="space-y-4 text-sm text-slate-700">
              <div>
                <div className="text-xs uppercase tracking-wide text-slate-500">Active peers</div>
                <div className="text-3xl font-semibold text-slate-900">{onlinePeers}</div>
              </div>
              <div>
                <div className="text-xs uppercase tracking-wide text-slate-500">Peer identifiers</div>
                <ul className="space-y-2 font-mono text-xs break-all">
                  {peerList.map((peer) => (
                    <li key={peer}>{peer}</li>
                  ))}
                </ul>
              </div>
            </div>
          )}
        </Card>
      </div>

      <Card title="Diagnostics">
        <p className="text-sm text-slate-600">
          The RPC endpoint <code className="font-mono">/p2p/peers</code> provides the live peer inventory, while
          <code className="font-mono">/api/v1/network</code> exposes aggregated metrics. These values update every few seconds
          without relying on any mock data.
        </p>
      </Card>
    </div>
  )
}
