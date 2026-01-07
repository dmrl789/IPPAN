import Link from "next/link";
import SimpleTable from "@/components/tables/SimpleTable";
import { Card } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";
import { SourceBadge } from "@/components/common/SourceBadge";
import { RpcErrorBanner } from "@/components/common/RpcErrorBanner";
import { fetchRecentBlocks } from "@/lib/blocks";
import { formatTimestamp, shortenHash } from "@/lib/format";
import { IPPAN_RPC_BASE } from "@/lib/rpc";

export default async function BlocksPage() {
  const result = await fetchRecentBlocks();

  return (
    <div className="space-y-6">
      <PageHeader 
        title="Blocks" 
        description="Latest blocks from devnet /blocks"
        actions={
          <a
            href="/api/rpc/debug"
            target="_blank"
            rel="noreferrer"
            className="rounded-lg border border-slate-800 bg-slate-950 px-3 py-1.5 text-xs font-medium text-slate-200 hover:border-emerald-500/50 hover:text-emerald-100"
          >
            Debug RPC
          </a>
        }
      />
      <Card title="Recent blocks" headerSlot={<SourceBadge source={result.source} />}>
        {!result.ok && (
          <div className="mb-4">
            <RpcErrorBanner
              error={{
                error: result.error,
                errorCode: "errorCode" in result ? result.errorCode : undefined,
                rpcBase: IPPAN_RPC_BASE,
                path: "/blocks",
              }}
              context="Blocks"
              showDebugLink={true}
            />
          </div>
        )}
        {result.ok && result.blocks.length === 0 ? (
          <p className="text-sm text-slate-400">No blocks available yet from this DevNet.</p>
        ) : result.blocks.length > 0 ? (
          <SimpleTable
            data={result.blocks}
            columns={[
              {
                key: "id",
                header: "Block",
                render: (row) => (
                  <Link href={`/blocks/${row.id}`} className="font-semibold text-emerald-300">
                    #{row.id}
                  </Link>
                )
              },
              { key: "hash", header: "Hash", render: (row) => shortenHash(row.hash) },
              { key: "hashTimer", header: "HashTimer", render: (row) => shortenHash(row.hashTimer, 8) },
              { key: "timestamp", header: "Timestamp", render: (row) => formatTimestamp(row.timestamp) },
              { key: "txCount", header: "Txs" }
            ]}
          />
        ) : null}
      </Card>
    </div>
  );
}
