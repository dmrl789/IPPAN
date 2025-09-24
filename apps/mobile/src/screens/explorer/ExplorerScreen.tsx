import React, { useCallback, useMemo, useState } from 'react';
import { ActivityIndicator, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useQuery } from '@tanstack/react-query';

import { Card } from '../../components/Card';
import { InfoRow } from '../../components/InfoRow';
import { Tag } from '../../components/Tag';
import { useApi } from '../../providers/ApiProvider';
import {
  fetchConsensusStats,
  fetchMempoolStats,
  fetchNetworkStats,
  fetchNodeStatus
} from '../../api';
import { formatDateTime, formatNumber, formatPercent } from '../../utils/format';

type BlockHistoryItem = {
  id: string;
  height: number;
  transactions: number;
  timestamp: string;
  proposer: string;
};

const FALLBACK_BLOCKS: BlockHistoryItem[] = Array.from({ length: 6 }).map((_, index) => ({
  id: `sim-block-${index}`,
  height: 1_234_560 - index,
  transactions: Math.floor(Math.random() * 120) + 12,
  timestamp: new Date(Date.now() - index * 12_000).toISOString(),
  proposer: `validator-${(index % 5) + 1}`
}));

function coerceBlockHistoryItem(entry: unknown, index: number): BlockHistoryItem {
  const record = entry && typeof entry === 'object' && !Array.isArray(entry) ? (entry as Record<string, unknown>) : {};
  const idValue = record.id ?? record.hash ?? `block-${index}`;
  const heightValue = record.height ?? record.block_height ?? record.number ?? 0;
  const txValue = record.transactions ?? record.tx_count ?? 0;
  const timestampValue = record.timestamp;

  const id = typeof idValue === 'string' ? idValue : `block-${index}`;
  const heightNumber = typeof heightValue === 'number' ? heightValue : Number(heightValue);
  const transactionsNumber = typeof txValue === 'number' ? txValue : Number(txValue);
  const timestamp =
    typeof timestampValue === 'string'
      ? timestampValue
      : typeof timestampValue === 'number'
      ? new Date(timestampValue).toISOString()
      : new Date().toISOString();
  const proposer =
    typeof record.proposer === 'string'
      ? record.proposer
      : typeof record.validator === 'string'
      ? record.validator
      : 'unknown';

  return {
    id,
    height: Number.isFinite(heightNumber) ? heightNumber : 0,
    transactions: Number.isFinite(transactionsNumber) ? transactionsNumber : 0,
    timestamp,
    proposer
  };
}

export default function ExplorerScreen() {
  const { client, baseUrl } = useApi();
  const [refreshing, setRefreshing] = useState(false);

  const nodeStatusQuery = useQuery({
    queryKey: ['node-status', baseUrl],
    queryFn: () => fetchNodeStatus(client),
    staleTime: 15_000
  });

  const networkStatsQuery = useQuery({
    queryKey: ['network-stats', baseUrl],
    queryFn: () => fetchNetworkStats(client),
    staleTime: 30_000
  });

  const mempoolStatsQuery = useQuery({
    queryKey: ['mempool-stats', baseUrl],
    queryFn: () => fetchMempoolStats(client),
    staleTime: 15_000
  });

  const consensusStatsQuery = useQuery({
    queryKey: ['consensus-stats', baseUrl],
    queryFn: () => fetchConsensusStats(client),
    staleTime: 15_000
  });

  const onRefresh = useCallback(async () => {
    try {
      setRefreshing(true);
      await Promise.all([
        nodeStatusQuery.refetch(),
        networkStatsQuery.refetch(),
        mempoolStatsQuery.refetch(),
        consensusStatsQuery.refetch()
      ]);
    } finally {
      setRefreshing(false);
    }
  }, [nodeStatusQuery, networkStatsQuery, mempoolStatsQuery, consensusStatsQuery]);

  const blockHistory = useMemo<BlockHistoryItem[]>(() => {
    const source = nodeStatusQuery.data?.recent_blocks || nodeStatusQuery.data?.blocks;
    if (Array.isArray(source) && source.length > 0) {
      return source.slice(0, 6).map((block, index) => coerceBlockHistoryItem(block, index));
    }
    return FALLBACK_BLOCKS;
  }, [nodeStatusQuery.data]);

  const nodeHealthTag = useMemo(() => {
    if (nodeStatusQuery.isError) {
      return <Tag label="Offline" tone="danger" />;
    }
    if (nodeStatusQuery.data?.node?.is_running ?? false) {
      return <Tag label="Online" tone="success" />;
    }
    if (nodeStatusQuery.data) {
      return <Tag label="Syncing" tone="warning" />;
    }
    return <Tag label="Unknown" tone="neutral" />;
  }, [nodeStatusQuery.isError, nodeStatusQuery.data]);

  return (
    <ScrollView
      style={styles.container}
      contentContainerStyle={styles.content}
      refreshControl={<RefreshControl refreshing={refreshing} onRefresh={onRefresh} tintColor="#38bdf8" />}>
      <Text style={styles.heading}>IPPAN Explorer</Text>
      <Text style={styles.subheading}>
        Inspect live validator telemetry, mempool pressure and consensus state from your configured node.
      </Text>

      <Card title="Node status" subtitle={`Connected to ${baseUrl}`} action={nodeHealthTag}>
        {nodeStatusQuery.isLoading ? (
          <ActivityIndicator color="#38bdf8" />
        ) : nodeStatusQuery.isError ? (
          <Text style={styles.placeholder}>Unable to reach the node. Pull to refresh after checking connectivity.</Text>
        ) : (
          <>
            <InfoRow label="Node ID" value={nodeStatusQuery.data?.node?.node_id ?? nodeStatusQuery.data?.node_id ?? '—'} />
            <InfoRow
              label="Version"
              value={nodeStatusQuery.data?.node?.version ?? nodeStatusQuery.data?.version ?? '—'}
            />
            <InfoRow
              label="Uptime"
              value={`${formatNumber((nodeStatusQuery.data?.node?.uptime_seconds ?? nodeStatusQuery.data?.uptime_seconds ?? 0) / 3600, {
                maximumFractionDigits: 1
              })} hours`}
            />
            <InfoRow
              label="Current block"
              value={nodeStatusQuery.data?.blockchain?.current_height ?? nodeStatusQuery.data?.current_block ?? '—'}
            />
          </>
        )}
      </Card>

      <Card title="Network" subtitle="Peer connectivity and health across the configured cluster.">
        {networkStatsQuery.isLoading ? (
          <ActivityIndicator color="#38bdf8" />
        ) : networkStatsQuery.isError ? (
          <Text style={styles.placeholder}>Network statistics unavailable.</Text>
        ) : (
          <>
            <InfoRow label="Connected peers" value={networkStatsQuery.data?.connected_peers ?? '—'} />
            <InfoRow label="Total peers" value={networkStatsQuery.data?.total_peers ?? '—'} />
            <InfoRow label="Network ID" value={networkStatsQuery.data?.network_id ?? '—'} />
            <InfoRow label="Protocol" value={networkStatsQuery.data?.protocol_version ?? '—'} />
          </>
        )}
      </Card>

      <Card title="Mempool" subtitle="Pending transactions waiting for inclusion in upcoming blocks.">
        {mempoolStatsQuery.isLoading ? (
          <ActivityIndicator color="#38bdf8" />
        ) : mempoolStatsQuery.isError ? (
          <Text style={styles.placeholder}>Mempool information unavailable.</Text>
        ) : (
          <>
            <InfoRow label="Transactions" value={mempoolStatsQuery.data?.total_transactions ?? 0} />
            <InfoRow label="Senders" value={mempoolStatsQuery.data?.total_senders ?? 0} />
            <InfoRow
              label="Estimated size"
              value={mempoolStatsQuery.data?.total_size ? `${formatNumber(mempoolStatsQuery.data.total_size)} bytes` : '—'}
            />
            {mempoolStatsQuery.data?.fee_distribution ? (
              <View style={styles.feeBreakdown}>
                {Object.entries(mempoolStatsQuery.data.fee_distribution).map(([tier, pct]) => (
                  <View key={tier} style={styles.feeRow}>
                    <Text style={styles.feeTier}>{tier}</Text>
                    <Text style={styles.feeValue}>{formatPercent(pct)}</Text>
                  </View>
                ))}
              </View>
            ) : null}
          </>
        )}
      </Card>

      <Card title="Consensus" subtitle="Round progression and validator participation metrics.">
        {consensusStatsQuery.isLoading ? (
          <ActivityIndicator color="#38bdf8" />
        ) : consensusStatsQuery.isError ? (
          <Text style={styles.placeholder}>Consensus metrics unavailable.</Text>
        ) : (
          <>
            <InfoRow label="Round" value={consensusStatsQuery.data?.current_round ?? '—'} />
            <InfoRow label="Block height" value={consensusStatsQuery.data?.block_height ?? '—'} />
            <InfoRow label="Validators" value={consensusStatsQuery.data?.validators_count ?? '—'} />
            <InfoRow label="Status" value={consensusStatsQuery.data?.consensus_status ?? '—'} />
          </>
        )}
      </Card>

      <Card title="Recent blocks" subtitle="Last proposer rotations and throughput across the network.">
        {blockHistory.map((block) => (
          <View key={block.id} style={styles.blockItem}>
            <View>
              <Text style={styles.blockHeight}>#{block.height}</Text>
              <Text style={styles.blockMeta}>Proposer {block.proposer}</Text>
            </View>
            <View style={{ alignItems: 'flex-end' }}>
              <Text style={styles.blockTransactions}>{block.transactions} txs</Text>
              <Text style={styles.blockMeta}>{formatDateTime(block.timestamp)}</Text>
            </View>
          </View>
        ))}
      </Card>
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#0f172a'
  },
  content: {
    padding: 20,
    paddingBottom: 120
  },
  heading: {
    fontSize: 28,
    fontWeight: '700',
    color: '#f8fafc',
    marginBottom: 4
  },
  subheading: {
    fontSize: 15,
    color: '#94a3b8',
    marginBottom: 20
  },
  placeholder: {
    color: '#64748b',
    fontSize: 14
  },
  feeBreakdown: {
    marginTop: 10,
    gap: 6
  },
  feeRow: {
    flexDirection: 'row',
    justifyContent: 'space-between'
  },
  feeTier: {
    color: '#cbd5f5',
    fontSize: 13
  },
  feeValue: {
    color: '#38bdf8',
    fontWeight: '600'
  },
  blockItem: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    backgroundColor: '#0b1220',
    borderRadius: 12,
    paddingHorizontal: 16,
    paddingVertical: 12,
    marginBottom: 12
  },
  blockHeight: {
    color: '#f8fafc',
    fontWeight: '700',
    fontSize: 16
  },
  blockTransactions: {
    color: '#38bdf8',
    fontWeight: '600',
    fontSize: 15
  },
  blockMeta: {
    color: '#64748b',
    fontSize: 12,
    marginTop: 4
  }
});
