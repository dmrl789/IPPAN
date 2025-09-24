import { AxiosInstance, AxiosRequestConfig, isAxiosError } from 'axios';

export interface WalletBalance {
  account: string;
  balance: number;
  staked: number;
  nonce: number;
  rewards?: number;
  pendingTransactions?: string[];
}

export interface WalletTransaction {
  id: string;
  txHash?: string;
  from: string;
  to: string;
  amount: number;
  fee?: number;
  memo?: string;
  timestamp: string;
  type?: string;
  status?: string;
}

export interface SubmitPaymentInput {
  from: string;
  to: string;
  amount: number;
  fee: number;
  nonce: number;
  memo?: string;
  signature: string;
}

export interface SubmitPaymentResponse {
  success: boolean;
  txId?: string;
  message?: string;
}

export interface NodeStatus {
  node_id?: string;
  status?: string;
  current_block?: number;
  total_transactions?: number;
  network_peers?: number;
  uptime_seconds?: number;
  version?: string;
  recent_blocks?: Array<Record<string, unknown>>;
  blocks?: Array<Record<string, unknown>>;
  node?: {
    is_running: boolean;
    uptime_seconds: number;
    version: string;
    node_id: string;
  };
  network?: {
    connected_peers: number;
    known_peers: number;
    total_peers: number;
  };
  mempool?: {
    total_transactions: number;
    pending_transactions: number;
  };
  blockchain?: {
    current_height: number;
    total_blocks: number;
    total_transactions: number;
  };
}

export interface NetworkStats {
  total_peers: number;
  connected_peers: number;
  network_id?: string;
  protocol_version?: string;
  uptime_seconds?: number;
}

export interface MempoolStats {
  total_transactions: number;
  total_senders?: number;
  total_size?: number;
  fee_distribution?: Record<string, number>;
}

export interface ConsensusStats {
  current_round: number;
  validators_count: number;
  block_height: number;
  consensus_status: string;
}

export interface HealthStatus {
  status: string;
  timestamp?: string;
  network?: string;
}

export interface ModelAsset {
  id: string;
  owner: string;
  name?: string;
  description?: string;
  arch_id?: number;
  version?: number;
  weights_hash?: string;
  size_bytes?: number;
  license_id?: number;
  created_at?: { us: number; round_id: number };
}

export interface DatasetAsset {
  id: string;
  owner: string;
  name: string;
  description?: string;
  size_bytes?: number;
  created_at?: { us: number; round_id: number };
}

const GET_OPTIONS: AxiosRequestConfig = {
  timeout: 12_000
};

function toNumber(value: unknown): number {
  if (typeof value === 'number') {
    return value;
  }
  if (typeof value === 'string') {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }
  return 0;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return null;
}

function asArray(value: unknown): unknown[] | null {
  return Array.isArray(value) ? value : null;
}

function getString(record: Record<string, unknown>, key: string): string | undefined {
  const value = record[key];
  return typeof value === 'string' ? value : undefined;
}

function parseCreatedAt(value: unknown): { us: number; round_id: number } | undefined {
  const record = asRecord(value);
  if (!record) {
    return undefined;
  }
  const usValue = record.us;
  const roundValue = record.round_id;
  if (typeof usValue === 'number' && typeof roundValue === 'number') {
    return { us: usValue, round_id: roundValue };
  }
  return undefined;
}

function normaliseWalletBalance(payload: unknown, address: string): WalletBalance {
  const payloadRecord = asRecord(payload);
  const dataRecord = payloadRecord?.data ? asRecord(payloadRecord.data) ?? payloadRecord : payloadRecord;

  if (!dataRecord) {
    return {
      account: address,
      balance: 0,
      staked: 0,
      nonce: 0
    };
  }

  const account = getString(dataRecord, 'account') ?? getString(dataRecord, 'address') ?? address;
  const pendingRaw = dataRecord.pending_transactions;
  const pendingTransactions = Array.isArray(pendingRaw)
    ? pendingRaw.map((item) => String(item))
    : undefined;

  return {
    account,
    balance: toNumber(dataRecord.balance ?? dataRecord.available ?? 0),
    staked: toNumber(dataRecord.staked ?? dataRecord.staked_amount ?? 0),
    rewards: toNumber(dataRecord.rewards ?? 0),
    nonce: toNumber(dataRecord.nonce ?? 0),
    pendingTransactions
  };
}

export async function fetchWalletBalance(client: AxiosInstance, address: string): Promise<WalletBalance> {
  try {
    const { data } = await client.get(`/api/v1/balance/${address}`, GET_OPTIONS);
    return normaliseWalletBalance(data, address);
  } catch (error) {
    if (isAxiosError(error) && error.response?.status === 404) {
      return normaliseWalletBalance({}, address);
    }
    const { data } = await client.get('/api/v1/balance', {
      ...GET_OPTIONS,
      params: { address }
    });
    return normaliseWalletBalance(data, address);
  }
}

export async function fetchWalletTransactions(client: AxiosInstance, address: string): Promise<WalletTransaction[]> {
  const { data } = await client.get('/api/v1/transactions', {
    ...GET_OPTIONS,
    params: { address }
  });

  const dataRecord = asRecord(data);
  const rows = asArray(data) ?? asArray(dataRecord?.transactions) ?? [];

  return rows.map((entry, index) => {
    const row = asRecord(entry) ?? {};
    return {
      id: String(row.id ?? row.tx_hash ?? `tx-${index}`),
      txHash: row.tx_hash ? String(row.tx_hash) : undefined,
      from: String(row.from ?? row.sender ?? ''),
      to: String(row.to ?? row.recipient ?? ''),
      amount: toNumber(row.amount ?? row.value ?? 0),
      fee: toNumber(row.fee ?? row.fee_paid ?? 0),
      memo: row.memo ? String(row.memo) : undefined,
      timestamp: row.timestamp ? String(row.timestamp) : new Date().toISOString(),
      type: row.type ? String(row.type) : undefined,
      status: row.status ? String(row.status) : undefined
    };
  });
}

export async function submitPayment(client: AxiosInstance, payload: SubmitPaymentInput): Promise<SubmitPaymentResponse> {
  try {
    const { data } = await client.post('/api/v1/transaction', payload);
    const success = Boolean(data?.success ?? data?.ok ?? data?.status === 'ok');
    const txIdCandidate =
      data?.data?.tx_hash ??
      data?.tx_id ??
      data?.hash ??
      data?.txHash ??
      data?.transaction_hash;
    return {
      success,
      txId: txIdCandidate ? String(txIdCandidate) : undefined,
      message: data?.message || data?.error
    };
  } catch (error) {
    if (isAxiosError(error)) {
      return {
        success: false,
        message: error.response?.data?.error || error.response?.data?.message || error.message
      };
    }
    return { success: false, message: 'Failed to submit transaction' };
  }
}

export type DomainRecord = Record<string, unknown> & {
  name?: string;
  domain?: string;
  owner?: string;
  expires_at?: string | number;
};

export async function fetchDomains(client: AxiosInstance): Promise<DomainRecord[]> {
  const { data } = await client.get('/api/domains', GET_OPTIONS);
  const dataRecord = asRecord(data);
  const rows = asArray(data) ?? asArray(dataRecord?.domains) ?? [];
  return rows.map((entry) => (asRecord(entry) ?? {}) as DomainRecord);
}

export async function fetchNodeStatus(client: AxiosInstance): Promise<NodeStatus> {
  const { data } = await client.get('/api/v1/status', GET_OPTIONS);
  return data;
}

export async function fetchNetworkStats(client: AxiosInstance): Promise<NetworkStats> {
  const { data } = await client.get('/api/v1/network', GET_OPTIONS);
  return data;
}

export async function fetchMempoolStats(client: AxiosInstance): Promise<MempoolStats> {
  const { data } = await client.get('/api/v1/mempool', GET_OPTIONS);
  return data;
}

export async function fetchConsensusStats(client: AxiosInstance): Promise<ConsensusStats> {
  const { data } = await client.get('/api/v1/consensus', GET_OPTIONS);
  return data;
}

export async function fetchHealth(client: AxiosInstance): Promise<HealthStatus> {
  const { data } = await client.get('/health', GET_OPTIONS);
  if (typeof data === 'string') {
    return { status: data };
  }
  return data;
}

export async function fetchModels(client: AxiosInstance): Promise<ModelAsset[]> {
  const { data } = await client.get('/api/models', GET_OPTIONS);
  const dataRecord = asRecord(data);
  const rows = asArray(data) ?? asArray(dataRecord?.models) ?? [];
  return rows.map((entry, index) => {
    const record = asRecord(entry) ?? {};
    const size = toNumber(record.size_bytes ?? record.sizeBytes ?? 0);
    return {
      id: typeof record.id === 'string' ? record.id : `model-${index}`,
      owner: typeof record.owner === 'string' ? record.owner : '',
      name: typeof record.name === 'string' ? record.name : undefined,
      description: typeof record.description === 'string' ? record.description : undefined,
      arch_id: typeof record.arch_id === 'number' ? record.arch_id : undefined,
      version: typeof record.version === 'number' ? record.version : undefined,
      weights_hash: typeof record.weights_hash === 'string' ? record.weights_hash : undefined,
      size_bytes: size > 0 ? size : undefined,
      license_id: typeof record.license_id === 'number' ? record.license_id : undefined,
      created_at: parseCreatedAt(record.created_at)
    } satisfies ModelAsset;
  });
}

export async function fetchDatasets(client: AxiosInstance): Promise<DatasetAsset[]> {
  const { data } = await client.get('/api/datasets', GET_OPTIONS);
  const dataRecord = asRecord(data);
  const rows = asArray(data) ?? asArray(dataRecord?.datasets) ?? [];
  return rows.map((entry, index) => {
    const record = asRecord(entry) ?? {};
    const size = toNumber(record.size_bytes ?? record.sizeBytes ?? 0);
    return {
      id: typeof record.id === 'string' ? record.id : `dataset-${index}`,
      owner: typeof record.owner === 'string' ? record.owner : '',
      name: typeof record.name === 'string' ? record.name : `Dataset #${index + 1}`,
      description: typeof record.description === 'string' ? record.description : undefined,
      size_bytes: size > 0 ? size : undefined,
      created_at: parseCreatedAt(record.created_at)
    } satisfies DatasetAsset;
  });
}
