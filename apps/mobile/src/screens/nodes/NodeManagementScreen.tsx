import React, { useEffect, useMemo, useState } from 'react';
import { ActivityIndicator, Alert, ScrollView, StyleSheet, Text, TextInput, View } from 'react-native';
import { useQuery } from '@tanstack/react-query';

import { Card } from '../../components/Card';
import { Button } from '../../components/Button';
import { Tag } from '../../components/Tag';
import { InfoRow } from '../../components/InfoRow';
import { useApi } from '../../providers/ApiProvider';
import { fetchHealth, fetchNodeStatus } from '../../api';
import { defaultNodes } from '../../data/nodes';
import { formatDateTime, formatNumber } from '../../utils/format';

export default function NodeManagementScreen() {
  const { baseUrl, setBaseUrl, client } = useApi();
  const [draftUrl, setDraftUrl] = useState(baseUrl);

  useEffect(() => {
    setDraftUrl(baseUrl);
  }, [baseUrl]);

  const healthQuery = useQuery({
    queryKey: ['health', baseUrl],
    queryFn: () => fetchHealth(client),
    staleTime: 30_000
  });

  const nodeStatusQuery = useQuery({
    queryKey: ['node-status-summary', baseUrl],
    queryFn: () => fetchNodeStatus(client),
    staleTime: 30_000
  });

  const connectionTag = useMemo(() => {
    if (healthQuery.isError || nodeStatusQuery.isError) {
      return <Tag label="Offline" tone="danger" />;
    }
    if (healthQuery.data?.status === 'ok' || nodeStatusQuery.data?.node?.is_running) {
      return <Tag label="Online" tone="success" />;
    }
    if (healthQuery.isLoading || nodeStatusQuery.isLoading) {
      return <Tag label="Checking" tone="info" />;
    }
    return <Tag label="Unknown" tone="neutral" />;
  }, [healthQuery.data, healthQuery.isError, healthQuery.isLoading, nodeStatusQuery.data, nodeStatusQuery.isError, nodeStatusQuery.isLoading]);

  const applyBaseUrl = () => {
    if (!draftUrl.trim()) {
      Alert.alert('Invalid URL', 'Provide the base URL of an IPPAN validator or gateway.');
      return;
    }
    setBaseUrl(draftUrl);
  };

  const handleSelectNode = (url: string) => {
    setBaseUrl(url);
  };

  return (
    <ScrollView style={styles.container} contentContainerStyle={styles.content}>
      <Text style={styles.heading}>Node management</Text>
      <Text style={styles.subheading}>
        Pick the default validator or gateway for API calls, view health checks and latency hints for community nodes.
      </Text>

      <Card title="Active endpoint" subtitle="All mobile requests use this base URL." action={connectionTag}>
        <Text style={styles.label}>Current base URL</Text>
        <View style={styles.urlBox}>
          <Text style={styles.urlText}>{baseUrl}</Text>
        </View>

        <Text style={styles.label}>Override URL</Text>
        <TextInput
          value={draftUrl}
          onChangeText={setDraftUrl}
          placeholder="https://validator.your-node:8080"
          placeholderTextColor="#475569"
          style={styles.input}
          autoCapitalize="none"
          autoCorrect={false}
        />
        <Button label="Apply" onPress={applyBaseUrl} />
      </Card>

      <Card title="Health" subtitle="Status derived from the node /health endpoint.">
        {healthQuery.isLoading ? (
          <ActivityIndicator color="#38bdf8" />
        ) : healthQuery.isError ? (
          <Text style={styles.placeholder}>The selected node did not respond to health checks.</Text>
        ) : (
          <>
            <InfoRow label="Status" value={healthQuery.data?.status ?? '—'} />
            <InfoRow
              label="Last heartbeat"
              value={healthQuery.data?.timestamp ? formatDateTime(healthQuery.data.timestamp) : '—'}
            />
            <InfoRow label="Network" value={healthQuery.data?.network ?? '—'} />
          </>
        )}
      </Card>

      <Card title="Node snapshot" subtitle="High level metrics from the status endpoint.">
        {nodeStatusQuery.isLoading ? (
          <ActivityIndicator color="#38bdf8" />
        ) : nodeStatusQuery.isError ? (
          <Text style={styles.placeholder}>Unable to fetch node statistics.</Text>
        ) : (
          <>
            <InfoRow label="Node ID" value={nodeStatusQuery.data?.node?.node_id ?? nodeStatusQuery.data?.node_id ?? '—'} />
            <InfoRow label="Peers" value={nodeStatusQuery.data?.network?.connected_peers ?? '—'} />
            <InfoRow
              label="Block height"
              value={nodeStatusQuery.data?.blockchain?.current_height ?? nodeStatusQuery.data?.current_block ?? '—'}
            />
            <InfoRow
              label="Transactions"
              value={nodeStatusQuery.data?.blockchain?.total_transactions ?? nodeStatusQuery.data?.total_transactions ?? '—'}
            />
          </>
        )}
      </Card>

      <Card title="Recommended endpoints" subtitle="Validator and gateway options curated by the IPPAN team.">
        {defaultNodes.map((node) => (
          <View key={node.id} style={styles.nodeOption}>
            <View style={{ flex: 1 }}>
              <Text style={styles.nodeTitle}>{node.label}</Text>
              <Text style={styles.nodeUrl}>{node.baseUrl}</Text>
              <Text style={styles.nodeMeta}>
                {node.region} · {formatNumber(node.latencyMs)} ms latency {node.isCommunity ? '· Community operated' : ''}
              </Text>
            </View>
            <Button label="Use" onPress={() => handleSelectNode(node.baseUrl)} />
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
  label: {
    color: '#cbd5f5',
    fontSize: 13,
    marginTop: 8,
    marginBottom: 8
  },
  urlBox: {
    backgroundColor: '#0b1220',
    borderRadius: 12,
    paddingHorizontal: 16,
    paddingVertical: 12,
    borderWidth: 1,
    borderColor: '#1f2a44'
  },
  urlText: {
    color: '#f8fafc',
    fontSize: 14
  },
  input: {
    backgroundColor: '#0b1220',
    borderRadius: 12,
    borderWidth: 1,
    borderColor: '#1f2a44',
    paddingHorizontal: 14,
    paddingVertical: 12,
    color: '#f8fafc',
    fontSize: 15,
    marginBottom: 12
  },
  placeholder: {
    color: '#64748b',
    fontSize: 14
  },
  nodeOption: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: 12,
    backgroundColor: '#0b1220',
    borderRadius: 12,
    paddingHorizontal: 16,
    paddingVertical: 12,
    marginBottom: 12
  },
  nodeTitle: {
    color: '#f8fafc',
    fontWeight: '600',
    fontSize: 15
  },
  nodeUrl: {
    color: '#94a3b8',
    fontSize: 13,
    marginTop: 2
  },
  nodeMeta: {
    color: '#64748b',
    fontSize: 12,
    marginTop: 4
  }
});
