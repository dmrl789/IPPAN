import React, { useMemo } from 'react';
import { ActivityIndicator, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useQuery } from '@tanstack/react-query';

import { Card } from '../../components/Card';
import { Tag } from '../../components/Tag';
import { useApi } from '../../providers/ApiProvider';
import { fetchDatasets, fetchModels } from '../../api';
import { formatBytes, formatDateTime, formatNumber, truncateAddress } from '../../utils/format';

const FALLBACK_MODELS = [
  {
    id: 'model-sentiment',
    owner: 'i7f3e2aa90b91c0fe8dd7d1234567890abcdefabcdefabcdefabcdefabcdef',
    name: 'Sentiment-classifier-v2',
    version: 2,
    arch_id: 42,
    size_bytes: 86_000_000,
    created_at: { us: Date.now() * 1000, round_id: 9234 }
  },
  {
    id: 'model-vision',
    owner: 'i1acbd00112233445566778899aabbccddeeff00112233445566778899aabb',
    name: 'Vision-small-v5',
    version: 5,
    arch_id: 11,
    size_bytes: 210_000_000,
    created_at: { us: Date.now() * 1000 - 86_400_000_000, round_id: 9230 }
  }
];

const FALLBACK_DATASETS = [
  {
    id: 'dataset-lidar',
    owner: 'i4b8e2fa76b33221100ffeeddccbbaa99887766554433221100ffeeddccbbaa',
    name: 'Autonomous LiDAR frames',
    description: '4.5M annotated LiDAR frames with semantic classes',
    size_bytes: 3_200_000_000,
    created_at: { us: Date.now() * 1000 - 5_400_000_000, round_id: 9100 }
  },
  {
    id: 'dataset-voice',
    owner: 'i9000ddccbbaa11223344556677889900aabbccddeeff001122334455667788',
    name: 'Conversational speech 32kHz',
    description: 'Multilingual voice conversations with diarisation labels',
    size_bytes: 1_100_000_000,
    created_at: { us: Date.now() * 1000 - 8_640_000_000, round_id: 9050 }
  }
];

const INFERENCE_JOBS = [
  {
    id: 'job-1',
    model: 'Vision-small-v5',
    status: 'completed',
    latencyMs: 420,
    tokens: 2048,
    requester: 'i29f3949393aabbccddeeff11223344556677889900aabbccddeeff11223344',
    timestamp: new Date(Date.now() - 12 * 60 * 1000).toISOString()
  },
  {
    id: 'job-2',
    model: 'Sentiment-classifier-v2',
    status: 'running',
    latencyMs: 1180,
    tokens: 4096,
    requester: 'i1888997766554433221100ffeeddccbbaa0099887766554433221100ffeedd',
    timestamp: new Date(Date.now() - 4 * 60 * 1000).toISOString()
  }
];

const BID_MARKET = [
  { id: 'bid-1', dataset: 'Autonomous LiDAR frames', amount: 2400, bidder: 'validator-eu-1', status: 'leader' },
  { id: 'bid-2', dataset: 'Conversational speech 32kHz', amount: 1800, bidder: 'gateway-apac', status: 'pending' }
];

const PROOF_EVENTS = [
  { id: 'proof-1', type: 'ZK-SNARK', model: 'Vision-small-v5', status: 'verified', timestamp: new Date().toISOString() },
  { id: 'proof-2', type: 'Attestation', model: 'Sentiment-classifier-v2', status: 'pending', timestamp: new Date(Date.now() - 3_600_000).toISOString() }
];

export default function NeuralScreen() {
  const { client, baseUrl } = useApi();

  const modelsQuery = useQuery({
    queryKey: ['models', baseUrl],
    queryFn: () => fetchModels(client),
    staleTime: 60_000
  });

  const datasetsQuery = useQuery({
    queryKey: ['datasets', baseUrl],
    queryFn: () => fetchDatasets(client),
    staleTime: 60_000
  });

  const models = useMemo(() => {
    if (modelsQuery.isError || !modelsQuery.data || modelsQuery.data.length === 0) {
      return FALLBACK_MODELS;
    }
    return modelsQuery.data.slice(0, 6);
  }, [modelsQuery.data, modelsQuery.isError]);

  const datasets = useMemo(() => {
    if (datasetsQuery.isError || !datasetsQuery.data || datasetsQuery.data.length === 0) {
      return FALLBACK_DATASETS;
    }
    return datasetsQuery.data.slice(0, 6);
  }, [datasetsQuery.data, datasetsQuery.isError]);

  return (
    <ScrollView style={styles.container} contentContainerStyle={styles.content}>
      <Text style={styles.heading}>Neural marketplace</Text>
      <Text style={styles.subheading}>
        Manage AI models, datasets and inference jobs across the IPPAN decentralised compute fabric.
      </Text>

      <Card title="Models" subtitle="Latest on-chain assets available for inference and training.">
        {modelsQuery.isLoading ? (
          <ActivityIndicator color="#38bdf8" />
        ) : null}
        {models.map((model) => (
          <View key={model.id} style={styles.assetItem}>
            <View style={{ flex: 1 }}>
              <Text style={styles.assetTitle}>{model.name ?? model.id}</Text>
              <Text style={styles.assetSubtitle}>Owner {truncateAddress(model.owner)}</Text>
              <Text style={styles.assetMeta}>
                Size {formatBytes(model.size_bytes ?? 0)} · Round {model.created_at?.round_id ?? '—'}
              </Text>
            </View>
            <Tag label={`v${model.version ?? 1}`} tone="info" />
          </View>
        ))}
      </Card>

      <Card title="Datasets" subtitle="Curated data resources for federated training and fine-tuning.">
        {datasetsQuery.isLoading ? <ActivityIndicator color="#38bdf8" /> : null}
        {datasets.map((dataset) => (
          <View key={dataset.id} style={styles.assetItem}>
            <View style={{ flex: 1 }}>
              <Text style={styles.assetTitle}>{dataset.name ?? dataset.id}</Text>
              <Text style={styles.assetSubtitle}>Provider {truncateAddress(dataset.owner)}</Text>
              <Text style={styles.assetMeta}>{dataset.description ?? '—'}</Text>
              <Text style={styles.assetMeta}>Size {formatBytes(dataset.size_bytes ?? 0)}</Text>
            </View>
            <Tag label="Dataset" tone="neutral" />
          </View>
        ))}
      </Card>

      <Card title="Inference activity" subtitle="Recent jobs routed through the IPPAN inference mesh.">
        {INFERENCE_JOBS.map((job) => (
          <View key={job.id} style={styles.inferenceItem}>
            <View style={{ flex: 1 }}>
              <Text style={styles.assetTitle}>{job.model}</Text>
              <Text style={styles.assetSubtitle}>Requester {truncateAddress(job.requester)}</Text>
              <Text style={styles.assetMeta}>{formatDateTime(job.timestamp)}</Text>
            </View>
            <View style={{ alignItems: 'flex-end' }}>
              <Tag label={job.status === 'completed' ? 'Completed' : 'Running'} tone={job.status === 'completed' ? 'success' : 'warning'} />
              <Text style={styles.assetMeta}>{formatNumber(job.tokens)} tokens</Text>
              <Text style={styles.assetMeta}>{job.latencyMs} ms</Text>
            </View>
          </View>
        ))}
      </Card>

      <Card title="Bid marketplace" subtitle="Live bidding rounds for training and inference workloads.">
        {BID_MARKET.map((bid) => (
          <View key={bid.id} style={styles.inferenceItem}>
            <View style={{ flex: 1 }}>
              <Text style={styles.assetTitle}>{bid.dataset}</Text>
              <Text style={styles.assetSubtitle}>Bidder {bid.bidder}</Text>
              <Text style={styles.assetMeta}>Amount {formatNumber(bid.amount)} IPN</Text>
            </View>
            <Tag label={bid.status === 'leader' ? 'Leader' : 'Pending'} tone={bid.status === 'leader' ? 'success' : 'warning'} />
          </View>
        ))}
      </Card>

      <Card title="Proofs & attestations" subtitle="Cryptographic evidence for executions in the network.">
        {PROOF_EVENTS.map((event) => (
          <View key={event.id} style={styles.inferenceItem}>
            <View style={{ flex: 1 }}>
              <Text style={styles.assetTitle}>{event.model}</Text>
              <Text style={styles.assetSubtitle}>{event.type}</Text>
              <Text style={styles.assetMeta}>{formatDateTime(event.timestamp)}</Text>
            </View>
            <Tag label={event.status === 'verified' ? 'Verified' : 'Pending'} tone={event.status === 'verified' ? 'success' : 'warning'} />
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
  assetItem: {
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
  assetTitle: {
    color: '#f8fafc',
    fontWeight: '600',
    fontSize: 15
  },
  assetSubtitle: {
    color: '#94a3b8',
    fontSize: 13,
    marginTop: 2
  },
  assetMeta: {
    color: '#64748b',
    fontSize: 12,
    marginTop: 2
  },
  inferenceItem: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: '#0b1220',
    borderRadius: 12,
    paddingHorizontal: 16,
    paddingVertical: 12,
    marginBottom: 12
  }
});
