import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Card, Badge, LoadingSpinner } from '../../components/UI'
import { getNodeStatus, getRecentBlocks } from '../../lib/api'

export default function ContractsPage() {
  const nodeStatusQuery = useQuery({
    queryKey: ['node-status', 'contracts'],
    queryFn: getNodeStatus,
    refetchInterval: 20000,
  })

  const recentBlocksQuery = useQuery({
    queryKey: ['recent-blocks', 'contracts'],
    queryFn: () => getRecentBlocks(10),
    refetchInterval: 15000,
  })

  const latestBlocks = useMemo(() => recentBlocksQuery.data?.blocks ?? [], [recentBlocksQuery.data?.blocks])

  return (
    <div className="space-y-6">
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold text-slate-900">Smart Contracts</h1>
          <p className="text-sm text-slate-600">
            Contract analytics are powered by the data that the connected IPPAN node exposes. The current build of the node does
            not ship a contract registry, so the dashboard reflects the live chain state instead of mock examples.
          </p>
        </div>
        <Badge variant="success">Live RPC</Badge>
      </div>

      <Card title="Node Support">
        {nodeStatusQuery.isLoading ? (
          <div className="flex items-center justify-center h-32">
            <LoadingSpinner />
          </div>
        ) : nodeStatusQuery.isError ? (
          <div className="text-sm text-red-600">Unable to reach the node status endpoint.</div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm text-slate-700">
            <div>
              <div className="text-xs uppercase tracking-wide text-slate-500">Node ID</div>
              <div className="font-mono text-xs break-all">{nodeStatusQuery.data?.node?.node_id}</div>
            </div>
            <div>
              <div className="text-xs uppercase tracking-wide text-slate-500">Software Version</div>
              <div>{nodeStatusQuery.data?.node?.version}</div>
            </div>
            <div>
              <div className="text-xs uppercase tracking-wide text-slate-500">Latest Block Height</div>
              <div>{nodeStatusQuery.data?.blockchain?.current_height?.toLocaleString() ?? '—'}</div>
            </div>
            <div>
              <div className="text-xs uppercase tracking-wide text-slate-500">Total Transactions</div>
              <div>{nodeStatusQuery.data?.blockchain?.total_transactions?.toLocaleString() ?? '—'}</div>
            </div>
            <div className="md:col-span-2 text-sm text-slate-600">
              The RPC server does not yet expose contract deployment metadata. Once available, this view will stream the
              registry directly from the node instead of relying on placeholders.
            </div>
          </div>
        )}
      </Card>

      <Card title="Recent Blocks">
        {recentBlocksQuery.isLoading ? (
          <div className="flex items-center justify-center h-48">
            <LoadingSpinner />
          </div>
        ) : recentBlocksQuery.isError ? (
          <div className="text-sm text-red-600">Unable to load recent blocks.</div>
        ) : latestBlocks.length === 0 ? (
          <div className="text-sm text-slate-600">No blocks have been produced yet.</div>
        ) : (
          <div className="overflow-x-auto">
            <table className="min-w-full text-left text-sm">
              <thead className="bg-slate-100 text-slate-600 uppercase text-xs">
                <tr>
                  <th className="px-3 py-2">Height</th>
                  <th className="px-3 py-2">Hash</th>
                  <th className="px-3 py-2">Transactions</th>
                  <th className="px-3 py-2">Timestamp</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-200">
                {latestBlocks.map((block) => (
                  <tr key={block.hash}>
                    <td className="px-3 py-2 font-mono text-xs">{block.height.toLocaleString()}</td>
                    <td className="px-3 py-2 font-mono text-xs">{block.hash.slice(0, 18)}…</td>
                    <td className="px-3 py-2">{block.transaction_count}</td>
                    <td className="px-3 py-2">{new Date(Math.floor(block.timestamp_micros / 1000)).toLocaleString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>

      <Card title="Getting Ready">
        <p className="text-sm text-slate-600">
          Deploy the contract execution service on your node and expose its API through the gateway. Once the backend emits
          deployment artifacts, the interface will automatically render them here without any code changes.
        </p>
      </Card>
    </div>
  )
}
