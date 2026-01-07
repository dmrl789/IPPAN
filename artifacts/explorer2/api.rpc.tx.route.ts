import { NextRequest, NextResponse } from "next/server";
import { proxyRpcRequest } from "@/lib/rpcProxy";

export const dynamic = "force-dynamic";
export const revalidate = 0;

export async function GET(
  request: NextRequest,
  { params }: { params: { hash: string } }
) {
  const txHash = params.hash;
  
  if (!txHash) {
    return NextResponse.json(
      {
        ok: false,
        error: "missing_tx_hash",
        error_code: "MISSING_PARAM",
        detail: "Transaction hash is required",
        rpc_base: "",
        path: "/tx/[hash]",
        ts: Date.now(),
      },
      { status: 400 }
    );
  }

  const result = await proxyRpcRequest(`/tx/${encodeURIComponent(txHash)}`, {
    timeout: 4000,
  });

  if (!result.ok) {
    // 404 is a valid response (transaction not found)
    const status = result.status_code === 404 ? 404 : 502;
    return NextResponse.json(result, { status });
  }

  return NextResponse.json(result, { status: 200 });
}
