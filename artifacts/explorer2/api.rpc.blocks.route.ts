import { NextResponse } from "next/server";
import { proxyRpcRequest } from "@/lib/rpcProxy";

export const dynamic = "force-dynamic";
export const revalidate = 0;

export async function GET() {
  // Try native /blocks first (some RPCs implement this)
  const direct = await proxyRpcRequest("/blocks", { timeout: 4000 });
  if (direct.ok) {
    return NextResponse.json(direct, { status: 200 });
  }

  // DevNet L1 RPC currently does not expose /blocks.
  // Fallback: derive a "recent blocks" list from /tx/recent and then hydrate via /block/<hash>.
  if (direct.status_code !== 404) {
    return NextResponse.json(direct, { status: 502 });
  }

  const recentTx = await proxyRpcRequest("/tx/recent?limit=50", { timeout: 4000 });
  if (!recentTx.ok) {
    return NextResponse.json(recentTx, { status: 502 });
  }

  const rows = Array.isArray(recentTx.data) ? (recentTx.data as any[]) : [];
  const hashes: string[] = [];
  for (const r of rows) {
    const h = r?.included?.block_hash;
    if (typeof h === "string" && h.length > 0) hashes.push(h);
  }

  // de-dupe preserving order
  const seen = new Set<string>();
  const uniq: string[] = [];
  for (const h of hashes) {
    if (!seen.has(h)) {
      seen.add(h);
      uniq.push(h);
    }
  }

  // Hydrate up to N blocks to avoid N+1 explosion.
  const MAX = 20;
  const out: any[] = [];

  for (const hash of uniq.slice(0, MAX)) {
    const b = await proxyRpcRequest(`/block/${encodeURIComponent(hash)}`, { timeout: 5000 });
    if (!b.ok || !b.data) continue;

    const data: any = b.data;
    const header = data?.header ?? {};
    const block = data?.block ?? {};

    const blockHash = typeof header?.block_hash === "string" ? header.block_hash : typeof block?.id === "string" ? block.id : hash;
    const hashtimer = typeof header?.hashtimer === "string" ? header.hashtimer : typeof block?.hash_timer === "string" ? block.hash_timer : "";

    const tsUs =
      typeof header?.hashtimer_timestamp_us === "number"
        ? header.hashtimer_timestamp_us
        : typeof block?.timestamp === "number"
          ? block.timestamp
          : null;
    const tsIso = tsUs ? new Date(Math.floor(tsUs / 1000)).toISOString() : new Date().toISOString();

    const txCount =
      Array.isArray(data?.tx_ids) ? data.tx_ids.length :
      typeof block?.tx_count === "number" ? block.tx_count :
      Array.isArray(block?.transaction_hashes) ? block.transaction_hashes.length :
      0;

    out.push({
      id: blockHash, // we use hash as ID so /blocks/<id> resolves via /block/<hash>
      hash: blockHash,
      hashTimer: hashtimer,
      timestamp: tsIso,
      txCount,
      ippan_time_us: tsUs ? String(tsUs) : undefined,
      ippan_time_ms: tsUs ? Math.floor(tsUs / 1000) : undefined,
    });
  }

  return NextResponse.json(
    {
      ok: true,
      data: out,
      rpc_base: recentTx.rpc_base,
      path: "/blocks",
      status_code: 200,
      ts: Date.now(),
    },
    { status: 200 },
  );
}
