import { useCallback, useEffect, useRef, useState } from 'react';
import { Badge, Button, Card, LoadingSpinner } from '../../components/UI';
import { ChainBlock, fetchRecentBlocks } from '../../lib/api';

const formatTimestamp = (timestampUs: number) => {
  if (!timestampUs) {
    return '—';
  }

  const millis = Math.floor(timestampUs / 1_000);
  return new Date(millis).toLocaleString();
};

export default function LiveBlocksPage() {
  const [blocks, setBlocks] = useState<ChainBlock[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const cancelledRef = useRef(false);

  const loadBlocks = useCallback(
    async (initial: boolean) => {
      if (cancelledRef.current) {
        return;
      }

      initial ? setIsLoading(true) : setRefreshing(true);
      setError(null);

      try {
        const recentBlocks = await fetchRecentBlocks(12);
        if (!cancelledRef.current) {
          setBlocks(recentBlocks);
        }
      } catch (err) {
        if (!cancelledRef.current) {
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (!cancelledRef.current) {
          initial ? setIsLoading(false) : setRefreshing(false);
        }
      }
    },
    [],
  );

  useEffect(() => {
    cancelledRef.current = false;
    loadBlocks(true);
    const interval = setInterval(() => loadBlocks(false), 10_000);

    return () => {
      cancelledRef.current = true;
      clearInterval(interval);
    };
  }, [loadBlocks]);

  if (isLoading) {
    return (
      <div className="flex h-64 items-center justify-center">
        <LoadingSpinner />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Live Blocks</h1>
          <p className="text-sm text-gray-600">
            Real-time list of the most recent Layer 1 blocks produced by the validator set.
          </p>
        </div>
        <Badge variant="success">Live Network</Badge>
      </div>

      <div className="flex justify-end">
        <Button onClick={() => loadBlocks(false)} disabled={refreshing}>
          {refreshing ? 'Refreshing…' : 'Refresh'}
        </Button>
      </div>

      {error && (
        <div className="rounded-md border border-red-200 bg-red-50 p-4 text-sm text-red-700">{error}</div>
      )}

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
        {blocks.map((block) => (
          <Card key={block.height} title={`Block ${block.height}`}>
            <dl className="space-y-2 text-sm text-gray-700">
              <div className="flex justify-between">
                <dt className="text-gray-500">Timestamp</dt>
                <dd>{formatTimestamp(block.timestamp)}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-gray-500">Transactions</dt>
                <dd>{block.transactions.length}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-gray-500">Proposer</dt>
                <dd className="font-mono text-xs">{block.proposer.slice(0, 20)}…</dd>
              </div>
              <div className="space-y-1">
                <dt className="text-gray-500">HashTimer</dt>
                <dd className="font-mono text-xs break-all text-gray-700">{block.hashtimer}</dd>
              </div>
              <div className="space-y-1">
                <dt className="text-gray-500">Prev Hash</dt>
                <dd className="font-mono text-xs break-all text-gray-700">{block.previousHash}</dd>
              </div>
              <div className="space-y-1">
                <dt className="text-gray-500">Merkle Root</dt>
                <dd className="font-mono text-xs break-all text-gray-700">{block.merkleRoot}</dd>
              </div>
            </dl>
          </Card>
        ))}
      </div>
    </div>
  );
}
