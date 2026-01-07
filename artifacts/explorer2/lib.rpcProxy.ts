/**
 * Server-side RPC proxy utilities.
 * 
 * These functions are intended for use in Next.js API routes to proxy
 * requests to the IPPAN RPC gateway. This eliminates CORS issues and
 * allows for consistent error handling.
 */

import { IPPAN_RPC_BASE, DEFAULT_RPC_BASE } from "./rpc";

export interface RpcProxyResult<T = unknown> {
  ok: boolean;
  data?: T;
  error?: string;
  error_code?: string;
  detail?: string;
  rpc_base: string;
  path: string;
  status_code?: number;
  ts: number;
}

/**
 * Get the server-side RPC base URL.
 * Prefers IPPAN_RPC_BASE_URL (server-only) over NEXT_PUBLIC variants.
 */
export function getServerRpcBase(): string {
  // Server-side only env var (preferred)
  const serverBase = process.env.IPPAN_RPC_BASE_URL?.trim();
  if (serverBase) {
    return serverBase.endsWith("/") ? serverBase.slice(0, -1) : serverBase;
  }
  
  // Fall back to the shared base (which may use NEXT_PUBLIC vars)
  return IPPAN_RPC_BASE;
}

/**
 * Proxy a request to the RPC gateway with timeout and error normalization.
 */
export async function proxyRpcRequest<T = unknown>(
  path: string,
  options: {
    timeout?: number;
    method?: "GET" | "POST";
    body?: unknown;
  } = {}
): Promise<RpcProxyResult<T>> {
  const { timeout = 4000, method = "GET", body } = options;
  const rpcBase = getServerRpcBase();
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  const fullUrl = `${rpcBase}${normalizedPath}`;
  const ts = Date.now();

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  try {
    const fetchOptions: RequestInit = {
      method,
      signal: controller.signal,
      cache: "no-store",
      headers: {
        "Content-Type": "application/json",
        "Accept": "application/json",
      },
    };

    if (body && method === "POST") {
      fetchOptions.body = JSON.stringify(body);
    }

    const response = await fetch(fullUrl, fetchOptions);
    
    if (!response.ok) {
      return {
        ok: false,
        error: `rpc_http_error`,
        error_code: `HTTP_${response.status}`,
        detail: `${response.status} ${response.statusText}`,
        rpc_base: rpcBase,
        path: normalizedPath,
        status_code: response.status,
        ts,
      };
    }

    // Try to parse JSON
    let data: T;
    try {
      data = await response.json();
    } catch (parseError) {
      return {
        ok: false,
        error: "rpc_json_parse_error",
        error_code: "JSON_PARSE_ERROR",
        detail: "Response was not valid JSON",
        rpc_base: rpcBase,
        path: normalizedPath,
        status_code: response.status,
        ts,
      };
    }

    return {
      ok: true,
      data,
      rpc_base: rpcBase,
      path: normalizedPath,
      status_code: response.status,
      ts,
    };

  } catch (err: unknown) {
    const isAbort = err instanceof Error && err.name === "AbortError";
    const isNetworkError = err instanceof Error && (
      err.message.includes("ECONNREFUSED") ||
      err.message.includes("ENOTFOUND") ||
      err.message.includes("network")
    );

    return {
      ok: false,
      error: isAbort ? "rpc_timeout" : isNetworkError ? "rpc_unreachable" : "rpc_fetch_error",
      error_code: isAbort ? "TIMEOUT" : isNetworkError ? "NETWORK_ERROR" : "FETCH_ERROR",
      detail: isAbort 
        ? `Request timed out after ${timeout}ms`
        : err instanceof Error 
          ? err.message 
          : String(err),
      rpc_base: rpcBase,
      path: normalizedPath,
      ts,
    };
  } finally {
    clearTimeout(timeoutId);
  }
}

/**
 * Check if the RPC gateway is healthy by probing /status.
 */
export async function checkRpcHealth(): Promise<{
  ok: boolean;
  rpc_base: string;
  latency_ms: number;
  error?: string;
}> {
  const start = Date.now();
  const result = await proxyRpcRequest("/status", { timeout: 5000 });
  const latency_ms = Date.now() - start;

  return {
    ok: result.ok,
    rpc_base: result.rpc_base,
    latency_ms,
    error: result.ok ? undefined : result.detail,
  };
}
