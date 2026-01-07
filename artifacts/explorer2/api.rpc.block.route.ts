import { NextRequest, NextResponse } from "next/server";
import { proxyRpcRequest } from "@/lib/rpcProxy";

export const dynamic = "force-dynamic";
export const revalidate = 0;

export async function GET(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  const blockId = params.id;
  
  if (!blockId) {
    return NextResponse.json(
      {
        ok: false,
        error: "missing_block_id",
        error_code: "MISSING_PARAM",
        detail: "Block ID is required",
        rpc_base: "",
        path: "/blocks/[id]",
        ts: Date.now(),
      },
      { status: 400 }
    );
  }

  // DevNet L1 RPC exposes /block/<hash>
  const result = await proxyRpcRequest(`/block/${encodeURIComponent(blockId)}`, {
    timeout: 4000,
  });

  if (!result.ok) {
    // 404 is a valid response (block not found)
    const status = result.status_code === 404 ? 404 : 502;
    return NextResponse.json(result, { status });
  }

  return NextResponse.json(result, { status: 200 });
}
