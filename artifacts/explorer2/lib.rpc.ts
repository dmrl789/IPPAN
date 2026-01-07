function normalizeRpcBase(rawBase?: string): string | undefined {
  if (!rawBase) return undefined;
  const trimmed = rawBase.trim();
  if (!trimmed) return undefined;
  return trimmed.endsWith("/") ? trimmed.slice(0, -1) : trimmed;
}

export const DEFAULT_RPC_BASE = "http://188.245.97.41:8080"; // fallback DevNet node

/**
 * Check if we're running in a browser environment.
 * Used to determine whether to use the proxy or direct RPC.
 */
function isBrowser(): boolean {
  return typeof window !== "undefined";
}

function getRpcBaseFromEnv(): string | undefined {
  const rawBase =
    process.env.NEXT_PUBLIC_IPPAN_RPC_BASE ??
    process.env.NEXT_PUBLIC_NODE_RPC ??
    process.env.NEXT_PUBLIC_IPPAN_RPC_URL ??
    process.env.NEXT_PUBLIC_IPPAN_RPC ??
    process.env.NEXT_PUBLIC_IPPAN_RPC_FALLBACK ??
    process.env.NEXT_PUBLIC_RPC_URL ??
    process.env.NEXT_PUBLIC_EXPLORER_API ??
    process.env.VITE_NODE_RPC ??
    process.env.IPPAN_RPC_URL ??
    process.env.IPPAN_RPC_BASE;

  return normalizeRpcBase(rawBase);
}

function getRpcBase(): string {
  const env = getRpcBaseFromEnv();
  if (!env) {
    // Log in dev, but do NOT crash the app
    if (process.env.NODE_ENV !== "production") {
      // eslint-disable-next-line no-console
      console.warn("RPC base URL missing, using fallback:", DEFAULT_RPC_BASE);
    }
    return DEFAULT_RPC_BASE;
  }
  return env;
}

/**
 * Resolved RPC base URL used across the explorer.
 * This is always a non-empty string (falls back to a known DevNet node).
 * 
 * NOTE: This is the DIRECT HTTP URL. On the browser, you should use
 * the /api/rpc/* proxy instead to avoid mixed content issues.
 * Use getEffectiveRpcBase() for automatic handling.
 */
export const IPPAN_RPC_BASE = getRpcBase();

export function getEnvRpcBaseUrl(): string | undefined {
  return getRpcBaseFromEnv();
}

/**
 * Get the effective RPC base URL for the current environment.
 * - Browser: Returns "" (empty string) to use same-origin /api/rpc/* proxy
 * - Server: Returns the direct RPC base URL
 * 
 * This automatically handles mixed content issues by using the proxy
 * when running in the browser.
 */
export function getEffectiveRpcBase(): string {
  // In browser, use the proxy to avoid mixed content (HTTPS -> HTTP blocked)
  if (isBrowser()) {
    return "/api/rpc";
  }
  // On server, use direct RPC base
  return IPPAN_RPC_BASE;
}

/**
 * DevNet architecture note:
 * The Explorer connects to a SINGLE canonical RPC gateway (IPPAN_RPC_BASE).
 * The gateway aggregates data from all validators internally.
 * Direct validator probing is NOT supported from the frontend - validators
 * are not public RPC endpoints and are only reachable within the DevNet peer graph.
 *
 * Topology:
 *   Explorer (Vercel) → Single RPC Gateway (188.245.97.41:8080) → Internal DevNet
 */

const RPC_BASE_URL = IPPAN_RPC_BASE;

export class RpcError extends Error {
  status: number;
  statusText: string;
  path: string;
  base: string;

  constructor(message: string, opts: { status: number; statusText: string; path: string; base: string }) {
    super(message);
    this.name = "RpcError";
    this.status = opts.status;
    this.statusText = opts.statusText;
    this.path = opts.path;
    this.base = opts.base;
  }
}

export function requireRpcBaseUrl(): string {
  return RPC_BASE_URL;
}

export function buildRpcUrl(path: string): string {
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  const base = getEffectiveRpcBase();
  return `${base}${normalizedPath}`;
}

async function safeJsonFetchWithStatusInternal<T>(
  path: string,
  init?: RequestInit,
): Promise<{ status: number | null; data: T | null; url: string }> {
  // Use proxy in browser to avoid mixed content (HTTPS -> HTTP blocked)
  const base = getEffectiveRpcBase().replace(/\/+$/, "");
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  const url = `${base}${normalizedPath}`;

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 7000);

  try {
    const res = await fetch(url, {
      ...init,
      signal: init?.signal ?? controller.signal,
      cache: "no-store",
      headers: {
        "Content-Type": "application/json",
        ...(init?.headers ?? {}),
      },
    });

    if (!res.ok) {
      if (isBrowser()) {
        // eslint-disable-next-line no-console
        console.error(`RPC error ${res.status} for ${url}`);
      }
      return { status: res.status, data: null, url };
    }

    // When using the proxy, the response is wrapped in { ok, data, ... }
    // When using direct RPC, the response is the raw data
    let jsonData: unknown;
    try {
      jsonData = await res.json();
    } catch (err) {
      if (isBrowser()) {
        // eslint-disable-next-line no-console
        console.error("RPC JSON parse failed for", url, err);
      }
      return { status: res.status, data: null, url };
    }

    // Handle proxy response format
    if (isBrowser() && jsonData && typeof jsonData === "object" && "ok" in jsonData) {
      const proxyResponse = jsonData as { ok: boolean; data?: T; status_code?: number; error?: string };
      if (proxyResponse.ok && proxyResponse.data !== undefined) {
        return { status: proxyResponse.status_code ?? res.status, data: proxyResponse.data, url };
      }
      // Proxy returned error
      return { status: proxyResponse.status_code ?? res.status, data: null, url };
    }

    // Direct RPC response (server-side)
    return { status: res.status, data: jsonData as T, url };
  } catch (err) {
    if (isBrowser()) {
      // eslint-disable-next-line no-console
      console.error("RPC fetch failed for", url, err);
    }
    return { status: null, data: null, url };
  } finally {
    clearTimeout(timeout);
  }
}

export async function safeJsonFetchWithStatus<T>(
  path: string,
  init?: RequestInit,
): Promise<{ status: number | null; data: T | null }> {
  const { status, data } = await safeJsonFetchWithStatusInternal<T>(path, init);
  return { status, data };
}

export async function safeJsonFetch<T>(path: string, init?: RequestInit): Promise<T | null> {
  const { data } = await safeJsonFetchWithStatusInternal<T>(path, init);
  return data;
}

/**
 * Strict fetch helper that throws on RPC failures (kept for back-compat).
 * Prefer `safeJsonFetch`/`safeJsonFetchWithStatus` for UI stability.
 */
export async function rpcFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  const base = getEffectiveRpcBase();
  const { status, data } = await safeJsonFetchWithStatusInternal<T>(normalizedPath, init);

  if (data === null) {
    const code = status ?? 0;
    const statusText = status === null ? "FETCH_FAILED" : "ERROR";
    throw new RpcError(`RPC error ${code} ${statusText} for ${normalizedPath} (base=${base})`, {
      status: code,
      statusText,
      path: normalizedPath,
      base,
    });
  }

  return data;
}
