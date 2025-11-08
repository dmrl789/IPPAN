/**
 * Blockchain API
 * API endpoints for blockchain-related functionality
 */

import { apiClient, APIResponse } from '../api-client';

export interface Block {
  height: number;
  hash: string;
  previousHash: string;
  timestamp: number;
  transactions: Transaction[];
  validator: string;
}

export interface Transaction {
  hash: string;
  from: string;
  to: string;
  amount: number;
  fee: number;
  timestamp: number;
  status: 'pending' | 'confirmed' | 'failed';
}

export interface NetworkStats {
  blockHeight: number;
  totalTransactions: number;
  activeValidators: number;
  networkHashrate: number;
  averageBlockTime: number;
  totalSupply: number;
}

/**
 * Get latest blocks
 */
export async function getLatestBlocks(limit: number = 10): Promise<APIResponse<Block[]>> {
  return apiClient.get<Block[]>(`/api/blockchain/blocks/latest?limit=${limit}`);
}

/**
 * Get block by height
 */
export async function getBlock(height: number): Promise<APIResponse<Block>> {
  return apiClient.get<Block>(`/api/blockchain/blocks/${height}`);
}

/**
 * Get block by hash
 */
export async function getBlockByHash(hash: string): Promise<APIResponse<Block>> {
  return apiClient.get<Block>(`/api/blockchain/blocks/hash/${hash}`);
}

/**
 * Get latest transactions
 */
export async function getLatestTransactions(limit: number = 10): Promise<APIResponse<Transaction[]>> {
  return apiClient.get<Transaction[]>(`/api/blockchain/transactions/latest?limit=${limit}`);
}

/**
 * Get transaction by hash
 */
export async function getTransaction(hash: string): Promise<APIResponse<Transaction>> {
  return apiClient.get<Transaction>(`/api/blockchain/transactions/${hash}`);
}

/**
 * Get network statistics
 */
export async function getNetworkStats(): Promise<APIResponse<NetworkStats>> {
  return apiClient.get<NetworkStats>('/api/blockchain/stats');
}

/**
 * Send a transaction
 */
export async function sendTransaction(transaction: Partial<Transaction>): Promise<APIResponse<Transaction>> {
  return apiClient.post<Transaction>('/api/blockchain/transactions', transaction);
}

/**
 * Get address balance
 */
export async function getAddressBalance(address: string): Promise<APIResponse<{ balance: number }>> {
  return apiClient.get<{ balance: number }>(`/api/blockchain/addresses/${address}/balance`);
}

/**
 * Get address transactions
 */
export async function getAddressTransactions(
  address: string,
  limit: number = 10
): Promise<APIResponse<Transaction[]>> {
  return apiClient.get<Transaction[]>(`/api/blockchain/addresses/${address}/transactions?limit=${limit}`);
}
