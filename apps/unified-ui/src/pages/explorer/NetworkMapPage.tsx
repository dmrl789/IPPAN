import { useCallback, useEffect, useRef, useState } from 'react';
import { Badge, Button, Card, LoadingSpinner } from '../../components/UI';
import { fetchPeerList, getNetworkStats, NetworkStats } from '../../lib/api';

export default function NetworkMapPage() {
  const [stats, setStats] = useState<NetworkStats | null>(null);
  const [peers, setPeers] = useState<string[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const cancelledRef = useRef(false);

  const loadNetwork = useCallback(
    async (initial: boolean) => {
      if (cancelledRef.current) {
        return;
      }

      initial ? setIsLoading(true) : setRefreshing(true);
      setError(null);

      try {
        const [networkStats, peerList] = await Promise.all([getNetworkStats(), fetchPeerList()]);
        if (cancelledRef.current) {
          return;
        }

        setStats(networkStats);
        setPeers(peerList);
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
    loadNetwork(true);
    const interval = setInterval(() => loadNetwork(false), 15_000);

    return () => {
      cancelledRef.current = true;
      clearInterval(interval);
    };
  }, [loadNetwork]);

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
          <h1 className="text-2xl font-bold text-gray-900">Network Peers</h1>
          <p className="text-sm text-gray-600">
            Connected peers reported by the embedded HTTP P2P network service.
          </p>
        </div>
        <Badge variant="success">Live Network</Badge>
      </div>

      <div className="flex justify-end">
        <Button onClick={() => loadNetwork(false)} disabled={refreshing}>
          {refreshing ? 'Refreshing…' : 'Refresh'}
        </Button>
      </div>

      {error && (
        <div className="rounded-md border border-red-200 bg-red-50 p-4 text-sm text-red-700">{error}</div>
      )}

      {stats && (
        <Card title="Network Overview">
          <dl className="grid gap-4 md:grid-cols-2 lg:grid-cols-4 text-sm text-gray-700">
            <div>
              <dt className="text-gray-500">Connected Peers</dt>
              <dd className="text-lg font-semibold">{stats.connected_peers}</dd>
            </div>
            <div>
              <dt className="text-gray-500">Known Peers</dt>
              <dd className="text-lg font-semibold">{stats.total_peers}</dd>
            </div>
            <div>
              <dt className="text-gray-500">Network ID</dt>
              <dd className="font-mono text-xs">{stats.network_id}</dd>
            </div>
            <div>
              <dt className="text-gray-500">Protocol Version</dt>
              <dd className="font-mono text-xs">{stats.protocol_version}</dd>
            </div>
          </dl>
        </Card>
      )}

      <Card title={`Peers (${peers.length})`}>
        {peers.length === 0 ? (
          <div className="p-8 text-center text-gray-500">No peers reported.</div>
        ) : (
          <ul className="space-y-3 text-sm text-gray-700">
            {peers.map((peer) => (
              <li key={peer} className="rounded-md border border-gray-200 bg-white p-3 font-mono text-xs">
                {peer}
              </li>
            ))}
          </ul>
        )}
      </Card>
    </div>
  );
}
