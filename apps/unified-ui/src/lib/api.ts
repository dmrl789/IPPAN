import axios from 'axios';
import { UIConfig } from './config';

const STORAGE_KEY = 'ippan.api.baseUrl';
const DEFAULT_BASE_URL = UIConfig.apiBaseUrl;

let API_BASE_URL = DEFAULT_BASE_URL;

function persistBaseUrl(url: string) {
  if (typeof window === 'undefined') {
    return;
  }

  try {
    window.localStorage.setItem(STORAGE_KEY, url);
  } catch (error) {
    console.warn('Unable to persist API base URL:', error);
  }
}

function readStoredBaseUrl(): string | null {
  if (typeof window === 'undefined') {
    return null;
  }

  try {
    return window.localStorage.getItem(STORAGE_KEY);
  } catch (error) {
    console.warn('Unable to read stored API base URL:', error);
    return null;
  }
}

// Function to update API base URL
export function setApiBaseUrl(url: string) {
  API_BASE_URL = url;
  api.defaults.baseURL = API_BASE_URL;

  if (typeof window !== 'undefined') {
    (window as any).API_BASE_URL = API_BASE_URL;
    persistBaseUrl(API_BASE_URL);
  }
}

// Function to get current API base URL
export function getApiBaseUrl() {
  return API_BASE_URL;
}

export function buildApiUrl(path: string): string {
  if (!path) {
    return getApiBaseUrl();
  }

  const normalizedBase = getApiBaseUrl().replace(/\/$/, '');
  const normalizedPath = path.startsWith('/') ? path : `/${path}`;

  return `${normalizedBase}${normalizedPath}`;
}

export function initializeApiBaseUrl(): string {
  if (typeof window !== 'undefined') {
    const stored = readStoredBaseUrl();
    const baseUrl = stored || DEFAULT_BASE_URL;
    setApiBaseUrl(baseUrl);
    return baseUrl;
  }

  API_BASE_URL = DEFAULT_BASE_URL;
  return API_BASE_URL;
}

const api = axios.create({
  baseURL: DEFAULT_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

// Neural Network API
export interface ModelAsset {
  id: Uint8Array;
  owner: Uint8Array;
  arch_id: number;
  version: number;
  weights_hash: Uint8Array;
  size_bytes: number;
  train_parent: Uint8Array | null;
  train_config: number[];
  license_id: number;
  metrics: any[];
  provenance: any[];
  created_at: { us: number; round_id: number };
}

export async function postModel(model: ModelAsset): Promise<string> {
  const response = await api.post('/api/models', model);
  return response.data;
}

export async function getModels(): Promise<ModelAsset[]> {
  const response = await api.get('/api/models');
  return response.data;
}

// Dataset API
export interface Dataset {
  id: Uint8Array;
  owner: Uint8Array;
  name: string;
  description: string;
  size_bytes: number;
  created_at: { us: number; round_id: number };
}

export async function postDataset(dataset: Dataset): Promise<string> {
  const response = await api.post('/api/datasets', dataset);
  return response.data;
}

export async function getDatasets(): Promise<Dataset[]> {
  const response = await api.get('/api/datasets');
  return response.data;
}

// Accounts API
export interface AccountSummary {
  address: string;
  balance: number;
  nonce: number;
}

export async function getAccounts(): Promise<AccountSummary[]> {
  const response = await api.get('/accounts');
  const body = response.data;

  if (body?.success && Array.isArray(body.data)) {
    return body.data as AccountSummary[];
  }

  return [];
}

// Wallet API - Updated for real IPPAN nodes
export interface WalletBalance {
  address: string;
  balance: number;
  staked_amount: number;
  rewards: number;
  nonce: number;
  pending_transactions: string[];
  staked?: number;
}

export async function getWalletBalance(address: string): Promise<WalletBalance> {
  const response = await api.get(`/api/v1/balance/${address}`);
  return response.data;
}

export async function sendTransaction(transaction: any): Promise<string> {
  const response = await api.post('/api/v1/transaction', transaction);
  const body = response.data;

  if (body?.success && body?.data?.tx_hash) {
    return body.data.tx_hash as string;
  }

  const error = body?.error || 'Failed to submit transaction';
  throw new Error(error);
}

// Domain API
export interface Domain {
  name: string;
  owner: string;
  expires_at: number;
  price: number;
}

export async function getDomains(): Promise<Domain[]> {
  const response = await api.get('/api/domains');
  return response.data;
}

export async function registerDomain(domain: Partial<Domain>): Promise<string> {
  const response = await api.post('/api/domains', domain);
  return response.data;
}

// Storage API
export async function uploadFile(file: File): Promise<string> {
  const formData = new FormData();
  formData.append('file', file);
  
  const response = await api.post('/api/storage/upload', formData, {
    headers: {
      'Content-Type': 'multipart/form-data',
    },
  });
  return response.data;
}

// IPPAN Node API - Real backend endpoints
export interface NodeStatus {
  node_id: string;
  status: string;
  current_block: number;
  total_transactions: number;
  network_peers: number;
  uptime_seconds: number;
  version: string;
  node: {
    is_running: boolean;
    uptime_seconds: number;
    version: string;
    node_id: string;
  };
  network: {
    connected_peers: number;
    known_peers: number;
    total_peers: number;
  };
  mempool: {
    total_transactions: number;
    pending_transactions: number;
  };
  blockchain: {
    current_height: number;
    total_blocks: number;
    total_transactions: number;
  };
}

export interface NetworkStats {
  total_peers: number;
  connected_peers: number;
  network_id: string;
  protocol_version: string;
  uptime_seconds: number;
}

export interface MempoolStats {
  total_transactions: number;
  total_senders: number;
  total_size: number;
  fee_distribution: Record<string, number>;
}

export interface ConsensusStats {
  current_round: number;
  validators_count: number;
  block_height: number;
  consensus_status: string;
}

export interface ValidatorInfo {
  node_id: string;
  address: string;
  stake_amount: number;
  is_active: boolean;
}

export interface TransactionView {
  id: string;
  from: string;
  to: string;
  amount: number;
  nonce: number;
  timestamp: number;
  direction?: string;
  hashtimer: string;
}

export interface BlockSummary {
  height: number;
  hash: string;
  parent_hashes: string[];
  proposer: string;
  transaction_count: number;
  timestamp_micros: number;
}

export interface RecentBlocksResponse {
  latest_height: number;
  blocks: BlockSummary[];
}

export interface BlockDetail {
  height: number;
  hash: string;
  parent_hashes: string[];
  proposer: string;
  transaction_count: number;
  timestamp_micros: number;
  transactions: TransactionView[];
}

export interface BlockDetailResponse {
  block: BlockDetail;
}

// Node Status API
export async function getNodeStatus(): Promise<NodeStatus> {
  const response = await api.get('/api/v1/status');
  return response.data;
}

// Network API
export async function getNetworkStats(): Promise<NetworkStats> {
  const response = await api.get('/api/v1/network');
  return response.data;
}

export async function getP2PPeers(): Promise<string[]> {
  const response = await api.get('/p2p/peers');
  const peers = response.data?.peers;
  return Array.isArray(peers) ? peers : [];
}

// Mempool API
export async function getMempoolStats(): Promise<MempoolStats> {
  const response = await api.get('/api/v1/mempool');
  return response.data;
}

// Consensus API
export async function getConsensusStats(): Promise<ConsensusStats> {
  const response = await api.get('/api/v1/consensus');
  return response.data;
}

export async function getValidators(): Promise<ValidatorInfo[]> {
  const response = await api.get('/api/v1/validators');
  return response.data;
}

export async function getRecentBlocks(limit = 20): Promise<RecentBlocksResponse> {
  const response = await api.get('/api/v1/blocks/recent', {
    params: { limit },
  });
  return response.data;
}

export async function getBlockByHeight(height: number): Promise<BlockDetailResponse> {
  const response = await api.get(`/api/v1/blocks/${height}`);
  return response.data;
}

// Health Check
export async function getHealth(): Promise<any> {
  const response = await api.get('/health');
  return response.data;
}

export default api;
