import type { APIResponse } from './api-client';

const DEFAULT_RPC_BASE = 'http://localhost:18080';

function getRpcBase(): string {
  return (
    process.env.NEXT_PUBLIC_IPPAN_RPC_BASE ||
    // Fallback for existing Unified UI deployments.
    process.env.NEXT_PUBLIC_API_BASE_URL ||
    DEFAULT_RPC_BASE
  );
}

async function rpcRequest<T>(
  path: string,
  options: RequestInit = {}
): Promise<APIResponse<T>> {
  const base = getRpcBase().replace(/\/+$/, '');
  const url = `${base}${path.startsWith('/') ? '' : '/'}${path}`;

  try {
    const resp = await fetch(url, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...(options.headers || {}),
      },
    });

    if (!resp.ok) {
      const text = await resp.text().catch(() => 'Unknown error');
      return { status: resp.status, error: text || `HTTP ${resp.status}` };
    }

    const data = (await resp.json().catch(() => null)) as T;
    return { status: resp.status, data };
  } catch (err: any) {
    return { status: 0, error: err?.message || 'Network error' };
  }
}

export type RpcStatus = Record<string, any>;

export interface RpcTxRecentEntry {
  tx_id: string;
  status: string;
  included?: {
    block_hash: string;
    round_id: number;
    hashtimer: string;
  } | null;
  rejected_reason?: string | null;
  first_seen_ts?: number | null;
  tx_hashtimer?: string | null;
}

export async function rpcGetStatus(): Promise<APIResponse<RpcStatus>> {
  return rpcRequest<RpcStatus>('/status');
}

export async function rpcGetRecentTxs(
  limit: number = 50
): Promise<APIResponse<RpcTxRecentEntry[]>> {
  return rpcRequest<RpcTxRecentEntry[]>(`/tx/recent?limit=${limit}`);
}

export async function rpcGetTx(txId: string): Promise<APIResponse<any>> {
  return rpcRequest<any>(`/tx/${txId}`);
}

export async function rpcGetBlock(id: string): Promise<APIResponse<any>> {
  return rpcRequest<any>(`/block/${id}`);
}

export async function rpcGetRound(id: string): Promise<APIResponse<any>> {
  return rpcRequest<any>(`/round/${id}`);
}

