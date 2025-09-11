import axios from 'axios';

const API_BASE_URL = (import.meta as any).env?.VITE_API_URL || 'http://localhost:3000';

const api = axios.create({
  baseURL: API_BASE_URL,
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

// Wallet API
export interface WalletBalance {
  address: string;
  balance: number;
  staked: number;
  rewards: number;
}

export async function getWalletBalance(address: string): Promise<WalletBalance> {
  const response = await api.get(`/api/wallet/${address}/balance`);
  return response.data;
}

export async function sendTransaction(transaction: any): Promise<string> {
  const response = await api.post('/api/transactions', transaction);
  return response.data;
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

export default api;
