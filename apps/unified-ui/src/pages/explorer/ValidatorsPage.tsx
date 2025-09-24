import { useCallback, useEffect, useRef, useState } from 'react';
import { Badge, Button, Card, LoadingSpinner } from '../../components/UI';
import { fetchValidators, getConsensusStats, ConsensusStats, ValidatorInfo } from '../../lib/api';

export default function ValidatorsPage() {
  const [validators, setValidators] = useState<ValidatorInfo[]>([]);
  const [consensusStats, setConsensusStats] = useState<ConsensusStats | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const cancelledRef = useRef(false);

  const loadValidators = useCallback(
    async (initial: boolean) => {
      if (cancelledRef.current) {
        return;
      }

      initial ? setIsLoading(true) : setRefreshing(true);
      setError(null);

      try {
        const [validatorList, stats] = await Promise.all([fetchValidators(), getConsensusStats()]);
        if (cancelledRef.current) {
          return;
        }

        setValidators(validatorList);
        setConsensusStats(stats);
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
    loadValidators(true);
    const interval = setInterval(() => loadValidators(false), 12_000);

    return () => {
      cancelledRef.current = true;
      clearInterval(interval);
    };
  }, [loadValidators]);

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
          <h1 className="text-2xl font-bold text-gray-900">Validators</h1>
          <p className="text-sm text-gray-600">
            Active validators participating in the proof-of-authority consensus engine.
          </p>
        </div>
        <Badge variant="success">Live Network</Badge>
      </div>

      <div className="flex justify-end">
        <Button onClick={() => loadValidators(false)} disabled={refreshing}>
          {refreshing ? 'Refreshing…' : 'Refresh'}
        </Button>
      </div>

      {error && (
        <div className="rounded-md border border-red-200 bg-red-50 p-4 text-sm text-red-700">{error}</div>
      )}

      {consensusStats && (
        <Card title="Consensus Overview">
          <dl className="grid gap-4 md:grid-cols-2 lg:grid-cols-4 text-sm text-gray-700">
            <div>
              <dt className="text-gray-500">Current Round</dt>
              <dd className="text-lg font-semibold">{consensusStats.current_round}</dd>
            </div>
            <div>
              <dt className="text-gray-500">Block Height</dt>
              <dd className="text-lg font-semibold">{consensusStats.block_height}</dd>
            </div>
            <div>
              <dt className="text-gray-500">Validators</dt>
              <dd className="text-lg font-semibold">{consensusStats.validators_count}</dd>
            </div>
            <div>
              <dt className="text-gray-500">Status</dt>
              <dd className="text-lg font-semibold capitalize">{consensusStats.consensus_status}</dd>
            </div>
          </dl>
        </Card>
      )}

      <Card title={`Validators (${validators.length})`}>
        {validators.length === 0 ? (
          <div className="p-8 text-center text-gray-500">No validators configured.</div>
        ) : (
          <div className="overflow-x-auto">
            <table className="min-w-full text-left text-sm">
              <thead>
                <tr className="border-b bg-gray-50 text-xs uppercase tracking-wide text-gray-500">
                  <th className="px-4 py-3">Validator ID</th>
                  <th className="px-4 py-3">Address</th>
                  <th className="px-4 py-3 text-right">Stake</th>
                  <th className="px-4 py-3 text-center">Active</th>
                  <th className="px-4 py-3 text-center">Current Proposer</th>
                </tr>
              </thead>
              <tbody>
                {validators.map((validator) => (
                  <tr key={validator.id} className="border-b last:border-b-0">
                    <td className="px-4 py-3 font-mono text-xs text-gray-800">{validator.id}</td>
                    <td className="px-4 py-3 font-mono text-xs text-gray-800">{validator.address}</td>
                    <td className="px-4 py-3 text-right text-sm text-gray-700">{validator.stake.toLocaleString()}</td>
                    <td className="px-4 py-3 text-center text-sm text-gray-700">{validator.is_active ? 'Yes' : 'No'}</td>
                    <td className="px-4 py-3 text-center text-sm text-gray-700">{validator.is_proposer ? 'Yes' : 'No'}</td>
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
