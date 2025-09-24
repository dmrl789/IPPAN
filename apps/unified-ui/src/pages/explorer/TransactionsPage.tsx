import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Badge, Button, Card, Field, Input, LoadingSpinner } from '../../components/UI';
import { ChainTransaction, fetchRecentBlocks } from '../../lib/api';

type ExplorerTransaction = ChainTransaction & {
  blockHeight: number;
  blockHashTimer: string;
};

const formatTimestamp = (timestampUs: number) => {
  if (!timestampUs) {
    return '—';
  }

  const millis = Math.floor(timestampUs / 1_000);
  return new Date(millis).toLocaleString();
};

export default function TransactionsPage() {
  const navigate = useNavigate();
  const [transactions, setTransactions] = useState<ExplorerTransaction[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const cancelledRef = useRef(false);

  const loadTransactions = useCallback(
    async (initial: boolean) => {
      if (cancelledRef.current) {
        return;
      }

      initial ? setIsLoading(true) : setRefreshing(true);
      setError(null);

      try {
        const blocks = await fetchRecentBlocks(12);
        if (cancelledRef.current) {
          return;
        }

        const items = blocks
          .flatMap((block) =>
            block.transactions.map((tx) => ({
              ...tx,
              blockHeight: block.height,
              blockHashTimer: block.hashtimer,
            })),
          )
          .sort((a, b) => b.timestamp - a.timestamp);

        setTransactions(items);
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
    loadTransactions(true);

    const interval = setInterval(() => {
      loadTransactions(false);
    }, 15_000);

    return () => {
      cancelledRef.current = true;
      clearInterval(interval);
    };
  }, [loadTransactions]);

  const filteredTransactions = useMemo(() => {
    if (!search) {
      return transactions;
    }

    const needle = search.toLowerCase();
    return transactions.filter((tx) =>
      [tx.hash, tx.from, tx.to, String(tx.amount), tx.hashtimer]
        .filter(Boolean)
        .some((value) => value.toLowerCase().includes(needle)),
    );
  }, [transactions, search]);

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
          <h1 className="text-2xl font-bold text-gray-900">Transactions</h1>
          <p className="text-sm text-gray-600">
            Live view of the most recent Layer 1 transactions observed on the IPPAN chain.
          </p>
        </div>
        <Badge variant="success">Live Network</Badge>
      </div>

      <Card title="Search & Refresh">
        <div className="grid gap-4 md:grid-cols-[1fr_auto]">
          <Field label="Search by hash or address">
            <Input
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Enter hash, sender, or recipient"
            />
          </Field>
          <div className="flex items-end justify-end">
            <Button onClick={() => loadTransactions(false)} disabled={refreshing}>
              {refreshing ? 'Refreshing…' : 'Refresh'}
            </Button>
          </div>
        </div>
      </Card>

      {error && (
        <div className="rounded-md border border-red-200 bg-red-50 p-4 text-sm text-red-700">
          {error}
        </div>
      )}

      <Card title={`Recent Transactions (${filteredTransactions.length})`}>
        {filteredTransactions.length === 0 ? (
          <div className="p-8 text-center text-gray-500">No transactions found.</div>
        ) : (
          <div className="overflow-x-auto">
            <table className="min-w-full text-left text-sm">
              <thead>
                <tr className="border-b bg-gray-50 text-xs uppercase tracking-wide text-gray-500">
                  <th className="px-4 py-3">Hash</th>
                  <th className="px-4 py-3">Block</th>
                  <th className="px-4 py-3">Timestamp</th>
                  <th className="px-4 py-3">Participants</th>
                  <th className="px-4 py-3 text-right">Amount</th>
                  <th className="px-4 py-3">HashTimer</th>
                  <th className="px-4 py-3 text-right">Actions</th>
                </tr>
              </thead>
              <tbody>
                {filteredTransactions.map((tx) => (
                  <tr key={`${tx.hash}-${tx.blockHeight}`} className="border-b last:border-b-0">
                    <td className="px-4 py-3 font-mono text-xs text-gray-800">
                      {tx.hash.slice(0, 16)}…
                    </td>
                    <td className="px-4 py-3 text-sm text-gray-700">{tx.blockHeight}</td>
                    <td className="px-4 py-3 text-sm text-gray-700">{formatTimestamp(tx.timestamp)}</td>
                    <td className="px-4 py-3 text-sm text-gray-700">
                      <div className="font-mono text-xs">{tx.from.slice(0, 20)}…</div>
                      <div className="font-mono text-xs text-gray-500">→ {tx.to.slice(0, 20)}…</div>
                    </td>
                    <td className="px-4 py-3 text-right text-sm text-gray-700">{tx.amount.toLocaleString()}</td>
                    <td className="px-4 py-3 font-mono text-xs text-gray-600">{tx.hashtimer.slice(0, 20)}…</td>
                    <td className="px-4 py-3 text-right">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => navigate(`/explorer/live-blocks?height=${tx.blockHeight}`)}
                      >
                        View block
                      </Button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </div>
  );
}
