import type { BlockDetail, BlockSummary, Transaction } from "@/types/rpc";
import { safeJsonFetchWithStatus } from "@/lib/rpc";

function toIsoFromMaybeUs(value: any): string {
  if (typeof value === "number" && Number.isFinite(value)) {
    // DevNet uses microseconds in many places
    return new Date(Math.floor(value / 1000)).toISOString();
  }
  if (typeof value === "string") {
    // if already ISO (or parseable), keep
    const d = new Date(value);
    if (!Number.isNaN(d.getTime())) return d.toISOString();
  }
  return new Date().toISOString();
}

function normalizeTx(record: any, fallbackHash: string): Transaction {
  const amountAtomic =
    typeof record?.amountAtomic === "string"
      ? record.amountAtomic
      : typeof record?.amount_atomic === "string"
        ? record.amount_atomic
        : "0";
  const feeAtomic =
    typeof record?.feeAtomic === "string"
      ? record.feeAtomic
      : typeof record?.fee_atomic === "string"
        ? record.fee_atomic
        : "0";

  return {
    hash: typeof record?.hash === "string" ? record.hash : fallbackHash,
    from: typeof record?.from === "string" ? record.from : "",
    to: typeof record?.to === "string" ? record.to : "",
    // If `amount` isn't provided, fall back to atomic as a number for display (UI labels it IPN).
    amount: typeof record?.amount === "number" ? record.amount : Number.parseFloat(amountAtomic) || 0,
    amountAtomic,
    fee: typeof record?.fee === "number" ? record.fee : Number.parseFloat(feeAtomic) || 0,
    timestamp: toIsoFromMaybeUs(record?.timestamp),
    hashTimer: typeof record?.hashTimer === "string" ? record.hashTimer : typeof record?.hash_timer_id === "string" ? record.hash_timer_id : "",
    type: typeof record?.type === "string" ? record.type : "tx",
    status: typeof record?.status === "string" ? record.status : "unknown",
    blockId: typeof record?.blockId === "string" ? record.blockId : typeof record?.block_id === "string" ? record.block_id : undefined,
    ippan_time_us: typeof record?.ippan_time_us === "string" ? record.ippan_time_us : undefined,
    ippan_time_ms: typeof record?.ippan_time_ms === "number" ? record.ippan_time_ms : undefined
  };
}

function normalizeBlockSummary(record: any, fallbackId: string): BlockSummary {
  const id = typeof record?.id === "string" ? record.id : typeof record?.height === "number" ? String(record.height) : fallbackId;
  return {
    id,
    hash: typeof record?.hash === "string" ? record.hash : "",
    timestamp: toIsoFromMaybeUs(record?.timestamp),
    hashTimer: typeof record?.hashTimer === "string" ? record.hashTimer : typeof record?.hash_timer_id === "string" ? record.hash_timer_id : "",
    txCount: typeof record?.txCount === "number" ? record.txCount : typeof record?.tx_count === "number" ? record.tx_count : 0,
    ippan_time_us: typeof record?.ippan_time_us === "string" ? record.ippan_time_us : undefined,
    ippan_time_ms: typeof record?.ippan_time_ms === "number" ? record.ippan_time_ms : undefined
  };
}

function normalizeBlockDetail(record: any, fallbackId: string): BlockDetail {
  const summary = normalizeBlockSummary(record, fallbackId);
  const parents: string[] = Array.isArray(record?.parents) ? record.parents.filter((p: any) => typeof p === "string") : [];
  const rawTxs: any[] = Array.isArray(record?.transactions) ? record.transactions : [];
  const transactions = rawTxs.map((tx, idx) => normalizeTx(tx, `${summary.hash || "0x"}${idx}`));
  return { ...summary, parents, transactions };
}

export async function fetchRecentBlocks(): Promise<
  | { ok: true; source: "live"; blocks: BlockSummary[] }
  | { ok: false; source: "error"; error: string; errorCode?: string; blocks: BlockSummary[] }
> {
  const { status, data: payload } = await safeJsonFetchWithStatus<any>("/blocks");
  
  // DevNet may not expose /blocks endpoint yet (404)
  if (status === 404) {
    return {
      ok: false,
      source: "error",
      error: "Block list endpoint not available on this DevNet (404). The node is online but does not expose /blocks yet.",
      errorCode: "endpoint_not_available",
      blocks: []
    };
  }
  
  if (!payload) {
    return {
      ok: false,
      source: "error",
      error: "Gateway RPC unavailable (connection failed)",
      errorCode: "gateway_unreachable",
      blocks: []
    };
  }

  const rawBlocks: any[] = Array.isArray(payload) ? payload : Array.isArray(payload?.blocks) ? payload.blocks : [];
  return { ok: true, source: "live", blocks: rawBlocks.map((b, idx) => normalizeBlockSummary(b, String(idx))) };
}

export async function fetchBlockDetail(
  id: string
): Promise<
  | { ok: true; source: "live"; block: BlockDetail }
  | { ok: true; source: "live"; block: null }
  | { ok: false; source: "error"; error: string }
> {
  // DevNet L1 RPC exposes /block/<hash>
  const { status, data } = await safeJsonFetchWithStatus<any>(`/block/${encodeURIComponent(id)}`);

  if (status === 404) {
    return { ok: true, source: "live", block: null };
  }
  if (!data) {
    return { ok: false, source: "error", error: "Gateway RPC unavailable (connection failed)" };
  }

  // DevNet /block returns { block, header, tx_ids }. Normalize into explorer BlockDetail shape.
  const header = data?.header ?? {};
  const b = data?.block ?? data;
  const txs = Array.isArray(b?.transactions) ? b.transactions : [];
  const parents = Array.isArray(b?.parent_ids) ? b.parent_ids : Array.isArray(header?.parent_ids) ? header.parent_ids : [];

  const hash = typeof header?.block_hash === "string" ? header.block_hash : typeof b?.id === "string" ? b.id : id;
  const hashTimer = typeof header?.hashtimer === "string" ? header.hashtimer : typeof b?.hash_timer === "string" ? b.hash_timer : "";
  const tsIso = toIsoFromMaybeUs(header?.hashtimer_timestamp_us ?? b?.timestamp);

  const block: BlockDetail = {
    id: hash,
    hash,
    timestamp: tsIso,
    hashTimer,
    txCount: Array.isArray(data?.tx_ids) ? data.tx_ids.length : typeof b?.tx_count === "number" ? b.tx_count : txs.length,
    parents: parents.filter((p: any) => typeof p === "string"),
    transactions: txs.map((tx: any, idx: number) => normalizeTx(tx, `${hash}:${idx}`)),
    ippan_time_us: header?.hashtimer_timestamp_us ? String(header.hashtimer_timestamp_us) : undefined,
    ippan_time_ms: header?.hashtimer_timestamp_us ? Math.floor(header.hashtimer_timestamp_us / 1000) : undefined,
  };

  return { ok: true, source: "live", block };
}

