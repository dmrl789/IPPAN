import Link from "next/link";
import SimpleTable from "@/components/tables/SimpleTable";
import { SourceBadge } from "@/components/common/SourceBadge";
import { RpcErrorBanner } from "@/components/common/RpcErrorBanner";
import { Card } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";
import { HashTimerValue } from "@/components/common/HashTimerValue";
import { fetchBlockDetail } from "@/lib/blocks";
import { formatAmount, shortenHash } from "@/lib/format";
import { formatMs, toMsFromUs } from "@/lib/ippanTime";
import { IPPAN_RPC_BASE } from "@/lib/rpc";

interface BlockDetailPageProps {
  params: { id: string };
}

export default async function BlockDetailPage({ params }: BlockDetailPageProps) {
  const result = await fetchBlockDetail(params.id);
  
  // Handle RPC errors
  if (!result.ok) {
    return (
      <div className="space-y-6">
        <PageHeader 
          title="Block" 
          description={`#${params.id}`} 
          actions={<Link href="/blocks" className="text-sm text-slate-400 hover:text-slate-100">← Back to blocks</Link>} 
        />
        <Card title="DevNet RPC Error" headerSlot={<SourceBadge source="error" />}>
          <RpcErrorBanner
            error={{
              error: result.error,
              rpcBase: IPPAN_RPC_BASE,
              path: `/blocks/${params.id}`,
            }}
            context="Block Detail"
            showDebugLink={true}
          />
        </Card>
      </div>
    );
  }
  
  // Handle block not found (404 from DevNet)
  if (!result.block) {
    return (
      <div className="space-y-6">
        <PageHeader 
          title="Block Not Found" 
          description={`#${params.id}`} 
          actions={<Link href="/blocks" className="text-sm text-slate-400 hover:text-slate-100">← Back to blocks</Link>} 
        />
        <Card title="Block Not Found" headerSlot={<SourceBadge source="live" />}>
          <div className="rounded-lg border border-amber-900/50 bg-amber-950/30 p-4">
            <p className="text-sm text-amber-200/80">
              Block #{params.id} was not found on this DevNet.
            </p>
            <p className="mt-2 text-xs text-slate-500">
              This could mean the block ID is incorrect, or the block hasn&apos;t been created yet.
            </p>
          </div>
        </Card>
      </div>
    );
  }
  const resolvedBlock = result.block;
  const blockIppanMs =
    resolvedBlock.ippan_time_ms ??
    (resolvedBlock.ippan_time_us
      ? toMsFromUs(resolvedBlock.ippan_time_us)
      : new Date(resolvedBlock.timestamp).getTime());
  const blockIso = formatMs(blockIppanMs);

  return (
    <div className="space-y-6">
      <PageHeader
        title={`Block #${resolvedBlock.id}`}
        description={`Hash ${shortenHash(resolvedBlock.hash)}`}
        actions={
          <Link href="/blocks" className="text-sm text-slate-400 hover:text-slate-100">
            ← Back to blocks
          </Link>
        }
      />

      <Card title="Block header" headerSlot={<SourceBadge source={result.source} />}>
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <div>
              <p className="text-xs uppercase text-slate-500">IPPAN Time (ms)</p>
              <p className="text-lg font-semibold text-slate-50">{blockIppanMs.toLocaleString()}</p>
              <p className="text-xs text-slate-400">UTC {blockIso}</p>
            </div>
            <div>
              <p className="text-xs uppercase text-slate-500">HashTimer</p>
              <HashTimerValue
                id={resolvedBlock.hashTimer}
                linkClassName="font-mono text-sm text-emerald-300 underline-offset-4 hover:underline"
              />
            </div>
          </div>
          <div>
            <p className="text-xs uppercase text-slate-500">Parents</p>
            <div className="text-sm text-slate-200">
              {resolvedBlock.parents.map((parent) => (
                <div key={parent}>{parent}</div>
              ))}
            </div>
          </div>
        </div>
      </Card>

      <Card title="Transactions" description={`${resolvedBlock.transactions.length} transactions`}>
        <SimpleTable
          data={resolvedBlock.transactions}
          columns={[
            {
              key: "hash",
              header: "Tx",
              render: (row) => (
                <Link href={`/tx/${row.hash}`} className="text-emerald-300">
                  {shortenHash(row.hash)}
                </Link>
              )
            },
            { key: "from", header: "From", render: (row) => shortenHash(row.from) },
            { key: "to", header: "To", render: (row) => shortenHash(row.to) },
            { key: "amount", header: "Amount", render: (row) => formatAmount(row.amount) },
            { key: "fee", header: "Fee", render: (row) => `${row.fee} IPN` }
          ]}
        />
      </Card>
    </div>
  );
}
