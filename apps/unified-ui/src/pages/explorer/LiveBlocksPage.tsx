import { useEffect, useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Card, Badge, LoadingSpinner } from '../../components/UI'
import {
  BlockDetail,
  BlockSummary,
  BlockDetailResponse,
  getBlockByHeight,
  getRecentBlocks,
} from '../../lib/api'

const REFRESH_INTERVAL = 10000

function formatTimestamp(micros: number): string {
  if (!Number.isFinite(micros)) {
    return '—'
  }
  const millis = Math.floor(micros / 1000)
  return new Date(millis).toLocaleString()
}

function truncate(value: string, size = 10): string {
  if (!value) {
    return '—'
  }
  return value.length <= size ? value : `${value.slice(0, size)}…`
}

export default function LiveBlocksPage() {
  const [selectedHeight, setSelectedHeight] = useState<number | null>(null)

  const recentBlocksQuery = useQuery({
    queryKey: ['recent-blocks'],
    queryFn: () => getRecentBlocks(25),
    refetchInterval: REFRESH_INTERVAL,
  })

  const blockDetailQuery = useQuery<BlockDetailResponse>({
    queryKey: ['block-detail', selectedHeight],
    queryFn: () => getBlockByHeight(selectedHeight!),
    enabled: selectedHeight !== null,
    refetchInterval: REFRESH_INTERVAL,
  })

  const blocks: BlockSummary[] = recentBlocksQuery.data?.blocks ?? []

  useEffect(() => {
    if (!blocks.length) {
      return
    }
    if (selectedHeight === null) {
      setSelectedHeight(blocks[0].height)
    }
  }, [blocks, selectedHeight])

  const selectedBlock: BlockDetail | null = useMemo(() => {
    return blockDetailQuery.data?.block ?? null
  }, [blockDetailQuery.data])

  return (
    <div className="space-y-6">
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold text-slate-900">Live Blocks</h1>
          <p className="text-sm text-slate-600">
            Blocks are streamed directly from the connected IPPAN node. Data refreshes automatically every few seconds.
          </p>
        </div>
        <Badge variant="success">Live RPC</Badge>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <Card title="Recent Blocks" className="lg:col-span-2">
          {recentBlocksQuery.isLoading ? (
            <div className="flex items-center justify-center h-48">
              <LoadingSpinner />
            </div>
          ) : recentBlocksQuery.isError ? (
            <div className="text-sm text-red-600">
              Unable to load recent blocks. Ensure the node RPC service is reachable.
            </div>
          ) : blocks.length === 0 ? (
            <div className="text-sm text-slate-600">No blocks have been produced yet.</div>
          ) : (
            <div className="overflow-x-auto">
              <table className="min-w-full text-left text-sm">
                <thead className="bg-slate-100 text-slate-600 uppercase text-xs">
                  <tr>
                    <th className="px-3 py-2">Height</th>
                    <th className="px-3 py-2">Hash</th>
                    <th className="px-3 py-2">Transactions</th>
                    <th className="px-3 py-2">Proposer</th>
                    <th className="px-3 py-2">Timestamp</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-200">
                  {blocks.map((block) => (
                    <tr
                      key={block.hash}
                      className={`cursor-pointer transition-colors hover:bg-slate-50 ${
                        selectedHeight === block.height ? 'bg-slate-100' : ''
                      }`}
                      onClick={() => setSelectedHeight(block.height)}
                    >
                      <td className="px-3 py-2 font-mono text-xs">{block.height.toLocaleString()}</td>
                      <td className="px-3 py-2 font-mono text-xs">{truncate(block.hash, 16)}</td>
                      <td className="px-3 py-2">{block.transaction_count}</td>
                      <td className="px-3 py-2 font-mono text-xs">{truncate(block.proposer, 18)}</td>
                      <td className="px-3 py-2">{formatTimestamp(block.timestamp_micros)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Card>

        <Card title="Latest Height">
          {recentBlocksQuery.isLoading ? (
            <div className="flex items-center justify-center h-24">
              <LoadingSpinner />
            </div>
          ) : recentBlocksQuery.isError ? (
            <div className="text-sm text-red-600">Unable to determine the latest height.</div>
          ) : (
            <div className="space-y-2 text-sm text-slate-700">
              <div className="text-3xl font-semibold text-slate-900">
                {recentBlocksQuery.data?.latest_height?.toLocaleString() ?? '—'}
              </div>
              <p className="text-xs text-slate-500">
                Data served from <code className="font-mono">/api/v1/blocks/recent</code> on the connected node.
              </p>
            </div>
          )}
        </Card>
      </div>

      <Card title={selectedBlock ? `Block ${selectedBlock.height.toLocaleString()}` : 'Block Details'}>
        {selectedHeight === null ? (
          <div className="text-sm text-slate-600">Select a block to view its transactions and metadata.</div>
        ) : blockDetailQuery.isLoading ? (
          <div className="flex items-center justify-center h-48">
            <LoadingSpinner />
          </div>
        ) : blockDetailQuery.isError ? (
          <div className="text-sm text-red-600">
            Unable to load block details. The block may have been pruned or the node is unavailable.
          </div>
        ) : !selectedBlock ? (
          <div className="text-sm text-slate-600">No details were returned for the selected block.</div>
        ) : (
          <div className="space-y-6">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm text-slate-700">
              <div>
                <div className="text-xs uppercase tracking-wide text-slate-500">Hash</div>
                <div className="font-mono break-all text-xs">{selectedBlock.hash}</div>
              </div>
              <div>
                <div className="text-xs uppercase tracking-wide text-slate-500">Proposer</div>
                <div className="font-mono break-all text-xs">{selectedBlock.proposer}</div>
              </div>
              <div>
                <div className="text-xs uppercase tracking-wide text-slate-500">Timestamp</div>
                <div>{formatTimestamp(selectedBlock.timestamp_micros)}</div>
              </div>
              <div>
                <div className="text-xs uppercase tracking-wide text-slate-500">Transactions</div>
                <div>{selectedBlock.transaction_count}</div>
              </div>
              <div className="md:col-span-2">
                <div className="text-xs uppercase tracking-wide text-slate-500">Parent Hashes</div>
                <div className="font-mono break-all text-xs space-y-1">
                  {selectedBlock.parent_hashes.length === 0 ? (
                    <div className="text-slate-500">Genesis block</div>
                  ) : (
                    selectedBlock.parent_hashes.map((parent) => (
                      <div key={parent}>{parent}</div>
                    ))
                  )}
                </div>
              </div>
            </div>

            <div>
              <h2 className="text-lg font-semibold text-slate-900 mb-3">Transactions</h2>
              {selectedBlock.transactions.length === 0 ? (
                <div className="text-sm text-slate-600">No transactions were included in this block.</div>
              ) : (
                <div className="overflow-x-auto">
                  <table className="min-w-full text-left text-sm">
                    <thead className="bg-slate-100 text-slate-600 uppercase text-xs">
                      <tr>
                        <th className="px-3 py-2">Hash</th>
                        <th className="px-3 py-2">From</th>
                        <th className="px-3 py-2">To</th>
                        <th className="px-3 py-2">Amount</th>
                        <th className="px-3 py-2">Nonce</th>
                        <th className="px-3 py-2">Timestamp</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-slate-200">
                      {selectedBlock.transactions.map((tx) => (
                        <tr key={tx.id}>
                          <td className="px-3 py-2 font-mono text-xs">{truncate(tx.id, 18)}</td>
                          <td className="px-3 py-2 font-mono text-xs">{truncate(tx.from, 16)}</td>
                          <td className="px-3 py-2 font-mono text-xs">{truncate(tx.to, 16)}</td>
                          <td className="px-3 py-2">{tx.amount}</td>
                          <td className="px-3 py-2">{tx.nonce}</td>
                          <td className="px-3 py-2">{formatTimestamp(tx.timestamp)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          </div>
        )}
      </Card>
    </div>
  )
}
