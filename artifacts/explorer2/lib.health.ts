import type { HealthStatus } from "@/types/rpc";
import { safeJsonFetchWithStatus } from "@/lib/rpc";

function normalizeHealthStatus(input: unknown): HealthStatus | null {
  if (!input || typeof input !== "object") return null;
  const obj = input as Record<string, unknown>;

  // Newer L1 /health shape (as seen on current DevNet RPC)
  // {
  //   consensus_healthy: bool,
  //   rpc_healthy: bool,
  //   storage_healthy: bool,
  //   dht_healthy: bool,
  //   dht_file_mode: "stub" | "libp2p",
  //   dht_handle_mode: "stub" | "libp2p",
  //   ...
  // }
  if (
    typeof obj.consensus_healthy === "boolean" ||
    typeof obj.rpc_healthy === "boolean" ||
    typeof obj.storage_healthy === "boolean"
  ) {
    const dhtHealthy = typeof obj.dht_healthy === "boolean" ? obj.dht_healthy : true;
    const dhtFileMode = (obj.dht_file_mode === "libp2p" ? "libp2p" : "stub") as "stub" | "libp2p";
    const dhtHandleMode = (obj.dht_handle_mode === "libp2p" ? "libp2p" : "stub") as "stub" | "libp2p";

    return {
      consensus: Boolean(obj.consensus_healthy),
      rpc: Boolean(obj.rpc_healthy),
      storage: Boolean(obj.storage_healthy),
      dhtFile: { mode: dhtFileMode, healthy: dhtHealthy },
      dhtHandle: { mode: dhtHandleMode, healthy: dhtHealthy },
    };
  }

  // Legacy explorer shape (already normalized)
  if (
    typeof obj.consensus === "boolean" &&
    typeof obj.rpc === "boolean" &&
    typeof obj.storage === "boolean" &&
    typeof obj.dhtFile === "object" &&
    typeof obj.dhtHandle === "object"
  ) {
    return obj as unknown as HealthStatus;
  }

  return null;
}

export async function fetchHealthWithSource(): Promise<
  | { ok: true; source: "live"; health: HealthStatus }
  | { ok: false; source: "error"; error: string; errorCode?: string; health: HealthStatus | null }
> {
  const { status, data } = await safeJsonFetchWithStatus<unknown>("/health");
  const health = normalizeHealthStatus(data);

  // 404 means endpoint not implemented yet
  if (status === 404) {
    return {
      ok: false,
      source: "error",
      error: "Health endpoint not available on this DevNet (404 expected)",
      errorCode: "endpoint_not_available",
      health: null,
    };
  }

  // No response means gateway is unreachable
  if (!health) {
    return {
      ok: false,
      source: "error",
      error: status === null
        ? "Gateway RPC unavailable (connection failed)"
        : "Health payload not recognized (schema mismatch)",
      errorCode: "gateway_unreachable",
      health: null,
    };
  }

  return { ok: true, source: "live", health };
}

