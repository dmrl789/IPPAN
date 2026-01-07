import JsonViewer from "@/components/common/JsonViewer";
import { Card } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";
import { SourceBadge } from "@/components/common/SourceBadge";
import { StatusPill } from "@/components/ui/StatusPill";
import { ValidatorSourceBadge, getValidatorSource } from "@/components/common/ValidatorSourceBadge";
import { ValidatorIdentityPanel } from "@/components/common/ValidatorIdentityPanel";
import { GatewayTruth } from "@/components/common/GatewayTruth";
import { IppanTimeCard } from "@/components/IppanTimeCard";
import { RawStatusViewer } from "@/components/RawStatusViewer";
import { fetchAiStatusWithSource } from "@/lib/ai";
import { fetchHealthWithSource } from "@/lib/health";
import { fetchStatusWithSource } from "@/lib/status";
import { fetchPeers } from "@/lib/peers";

function formatUptime(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours >= 24) {
    const days = Math.floor(hours / 24);
    const remainingHours = hours % 24;
    return `${days}d ${remainingHours}h ${minutes}m`;
  }
  return `${hours}h ${minutes}m`;
}

export default async function StatusPage() {
  const [healthRes, aiRes, statusRes, peersRes] = await Promise.all([
    fetchHealthWithSource(), 
    fetchAiStatusWithSource(), 
    fetchStatusWithSource(),
    fetchPeers(),
  ]);
  const healthSource = healthRes.ok ? healthRes.source : "error";
  const aiSource = aiRes.ok ? (aiRes.source === "missing" ? "live" : aiRes.source) : "error";
  const statusSource = statusRes.ok ? statusRes.source : "error";

  const status = statusRes.ok ? statusRes.status : undefined;
  const rawStatus = statusRes.ok ? statusRes.rawStatus : (statusRes.rawStatus ?? null);
  const ippanTime = statusRes.ok ? statusRes.ippanTime : null;
  const hashTimerData = statusRes.ok ? statusRes.hashTimerData : null;
  const validatorIds = status?.consensus?.validator_ids ?? [];
  const { source: validatorSource, count: validatorCount } = getValidatorSource(status);
  const peerCount = status?.peer_count ?? peersRes.peers.length;

  return (
    <div className="space-y-6">
      <PageHeader title="Status" description="Operator / cluster view (health + consensus + AI) from devnet RPC" />

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        <Card title="Node health" description="From /health" headerSlot={<SourceBadge source={healthSource} />}>
          {healthRes.ok && healthRes.health ? (
            <div className="grid gap-3 sm:grid-cols-2">
              <HealthRow label="Consensus" ok={healthRes.health.consensus} detail="consensus" />
              <HealthRow label="RPC" ok={healthRes.health.rpc} detail="rpc" />
              <HealthRow label="Storage" ok={healthRes.health.storage} detail="storage" />
              <HealthRow label="DHT files" ok={healthRes.health.dhtFile.healthy} detail={`mode: ${healthRes.health.dhtFile.mode}`} />
              <HealthRow label="DHT handles" ok={healthRes.health.dhtHandle.healthy} detail={`mode: ${healthRes.health.dhtHandle.mode}`} />
            </div>
          ) : (
            <div className="rounded-lg border border-slate-800/70 bg-slate-900/30 p-3">
              <p className="text-sm text-slate-400">
                {"errorCode" in healthRes && healthRes.errorCode === "endpoint_not_available"
                  ? "DevNet feature — Health endpoint not yet exposed"
                  : healthRes.ok
                    ? "Health unavailable."
                    : "Health data unavailable — gateway error"}
              </p>
              {!healthRes.ok && "errorCode" in healthRes && healthRes.errorCode !== "endpoint_not_available" && (
                <p className="mt-1 text-xs text-slate-500">{healthRes.error}</p>
              )}
            </div>
          )}
        </Card>

        <Card title="Node Status" description="From /status" headerSlot={<SourceBadge source={statusSource} />}>
          {status ? (
            <div className="space-y-3 text-sm text-slate-200">
              <div className="flex items-center justify-between">
                <span className="text-slate-400">Network Active</span>
                <StatusPill status={status.network_active ? "ok" : "warn"} />
              </div>
              <KeyValue label="Node ID" value={status.node_id} />
              <KeyValue label="Version" value={status.version} />
              <KeyValue label="Uptime" value={formatUptime(status.uptime_seconds)} />
              <KeyValue label="Peer Count" value={status.peer_count.toString()} />
              <KeyValue label="Mempool Size" value={status.mempool_size.toString()} />
              <KeyValue label="Consensus Round" value={`#${status.consensus.round}`} />
              <div className="space-y-1">
                <div className="flex items-center justify-between">
                  <span className="text-slate-400">Validators</span>
                  <span className="font-semibold text-slate-100">{validatorCount}</span>
                </div>
                <ValidatorSourceBadge source={validatorSource} />
              </div>
              <p className="text-xs text-slate-500">
                Status reflects this explorer&apos;s connected RPC node view.
              </p>
            </div>
          ) : (
            <div className="rounded-lg border border-slate-800/70 bg-slate-900/30 p-3">
              <p className="text-sm text-slate-400">
                {statusRes.ok ? "Status unavailable." : "Node status unavailable — gateway error"}
              </p>
              {!statusRes.ok && (
                <p className="mt-1 text-xs text-slate-500">
                  {"errorCode" in statusRes && statusRes.errorCode === "gateway_unreachable"
                    ? "Unable to reach the canonical RPC gateway."
                    : statusRes.error}
                </p>
              )}
            </div>
          )}
        </Card>

        <Card title="AI status" description="From /status AI section" headerSlot={<SourceBadge source={statusSource} />}>
          {status?.ai ? (
            <div className="space-y-2">
              <StatusPill status={status.ai.using_stub ? "warn" : "ok"} />
              <KeyValue label="AI Enabled" value={status.ai.enabled ? "Yes" : "No"} />
              <KeyValue label="Consensus Mode" value={status.ai.consensus_mode} />
              <KeyValue label="Using Stub" value={status.ai.using_stub ? "Yes" : "No"} />
              <KeyValue label="Model Version" value={status.ai.model_version} />
              <p className="text-sm text-slate-400 mt-2">Model hash</p>
              <p className="font-mono text-xs text-slate-50 break-all">{status.ai.model_hash}</p>
              {status.ai.shadow_loaded && (
                <>
                  <p className="text-sm text-slate-400 mt-2">Shadow model hash</p>
                  <p className="font-mono text-xs text-slate-50 break-all">{status.ai.shadow_model_hash}</p>
                </>
              )}
            </div>
          ) : !aiRes.ok ? (
            <p className="text-sm text-slate-400">{aiRes.error}</p>
          ) : aiRes.aiAvailable && aiRes.ai ? (
            <div className="space-y-2">
              <StatusPill status={aiRes.ai.usingStub ? "warn" : "ok"} />
              <p className="text-sm text-slate-400">Model hash</p>
              <p className="font-mono text-base text-slate-50 break-all">{aiRes.ai.modelHash}</p>
              <p className="text-xs text-slate-500">Mode: {aiRes.ai.mode}</p>
            </div>
          ) : (
            <p className="text-sm text-slate-400">{aiRes.source === "missing" ? aiRes.message : "AI status unavailable."}</p>
          )}
        </Card>
      </div>

      {/* Validator Identity Panel */}
      <Card 
        title="Validator Identities" 
        description="Validator IDs from /status with duplicate detection"
        headerSlot={<ValidatorSourceBadge source={validatorSource} />}
      >
        {status ? (
          <ValidatorIdentityPanel 
            validatorIds={validatorIds}
            selfId={status.consensus.self_id}
            peerCount={peerCount}
          />
        ) : (
          <div className="rounded-lg border border-slate-800/70 bg-slate-900/30 p-3">
            <p className="text-sm text-slate-400">
              {validatorSource === "unknown" 
                ? "Node RPC lacks validator visibility — consensus.validator_ids not available"
                : "Validator identity data unavailable — gateway error"}
            </p>
          </div>
        )}
      </Card>

      {/* Peer Identity Section */}
      <Card
        title="Peer Identities"
        description="Peer IDs from /peers endpoint"
        headerSlot={<SourceBadge source={peersRes.ok ? peersRes.source : "error"} />}
      >
        {peersRes.ok && peersRes.peers.length > 0 ? (
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              <span className="text-sm text-slate-400">Total peers:</span>
              <span className="text-sm font-semibold text-slate-100">{peersRes.peers.length}</span>
            </div>
            <div className="rounded-lg border border-slate-800/70 bg-slate-950/50 overflow-hidden">
              <div className="px-3 py-2 border-b border-slate-800/70">
                <span className="text-xs uppercase tracking-wide text-slate-500">Peer IDs</span>
              </div>
              <div className="divide-y divide-slate-800/50 max-h-48 overflow-y-auto">
                {peersRes.peers.map((peer, i) => (
                  <div key={peer.peer_id || i} className="flex items-center justify-between gap-2 px-3 py-2">
                    <span className="font-mono text-xs text-slate-300 truncate" title={peer.peer_id}>
                      {peer.peer_id ? `${peer.peer_id.slice(0, 16)}…${peer.peer_id.slice(-8)}` : "unknown"}
                    </span>
                    {peer.agent && (
                      <span className="text-[10px] text-slate-500 truncate max-w-[120px]" title={peer.agent}>
                        {peer.agent}
                      </span>
                    )}
                  </div>
                ))}
              </div>
            </div>
          </div>
        ) : (
          <div className="rounded-lg border border-slate-800/70 bg-slate-900/30 p-3">
            <p className="text-sm text-slate-400">
              {!peersRes.ok && peersRes.errorCode === "endpoint_not_available"
                ? "/peers endpoint not available on this DevNet"
                : peersRes.peers.length === 0
                  ? "/peers returned empty — no peer data available"
                  : !peersRes.ok 
                    ? peersRes.error 
                    : "Peer data unavailable"}
            </p>
          </div>
        )}
      </Card>

      {/* IPPAN Time Section - Dedicated time display */}
      <Card 
        title="Time & Ordering" 
        description="IPPAN Time, HashTimers, and ordering fields"
      >
        <div className="grid gap-4 md:grid-cols-2">
          <IppanTimeCard
            ippanTime={ippanTime}
            hashTimerData={hashTimerData}
            source={statusSource}
            loading={false}
          />
          
          {/* Time Info Panel */}
          <div className="space-y-3">
            <div className="rounded-lg border border-slate-800/70 bg-slate-950/50 p-4">
              <h4 className="text-sm font-semibold text-slate-200 mb-2">About IPPAN Time</h4>
              <p className="text-xs text-slate-400 mb-2">
                IPPAN uses HashTimers for ordering. Time fields in the /status response 
                indicate the network&apos;s view of time, which may differ from wall clock time.
              </p>
              <ul className="text-xs text-slate-500 space-y-1">
                <li>• <strong className="text-slate-400">IPPAN Time (μs)</strong>: Network time in microseconds</li>
                <li>• <strong className="text-slate-400">HashTimer</strong>: 64-char hex ordering anchor</li>
                <li>• <strong className="text-slate-400">Clock Delta</strong>: Difference from local time</li>
              </ul>
            </div>
            
            {/* Time Source Info */}
            {ippanTime?.sourceField && (
              <div className="rounded-lg border border-emerald-900/50 bg-emerald-950/30 p-3">
                <p className="text-xs text-emerald-300">
                  ✓ Time found in field: <code className="bg-slate-900/50 px-1 rounded">{ippanTime.sourceField}</code>
                </p>
              </div>
            )}
            
            {!ippanTime?.sourceField && (
              <div className="rounded-lg border border-amber-900/50 bg-amber-950/30 p-3">
                <p className="text-xs text-amber-300">
                  ⚠ No recognized time field found in /status response
                </p>
                <p className="text-xs text-slate-500 mt-1">
                  Check the &quot;All Fields&quot; view below for available time data
                </p>
              </div>
            )}
          </div>
        </div>
      </Card>

      {/* Gateway Truth Section */}
      <GatewayTruth 
        status={status}
        fetchTimestamp={Date.now()}
      />

      {/* Raw Status with All Fields View */}
      {rawStatus && (
        <RawStatusViewer
          data={rawStatus}
          title="Raw /status JSON"
          description="Complete response from gateway RPC - includes all fields (known and unknown)"
          defaultView="collapsed"
        />
      )}

      {/* Health and AI Raw Data */}
      <div className="grid gap-4 md:grid-cols-2">
        <Card title="Raw /health JSON">
          <JsonViewer data={healthRes.ok ? healthRes.health : { error: healthRes.error }} />
        </Card>
        <Card title="Raw /ai/status JSON">
          <JsonViewer
            data={
              aiRes.ok
                ? aiRes.aiAvailable
                  ? aiRes.ai
                  : { note: aiRes.source === "missing" ? aiRes.message : "AI status unavailable." }
                : { error: aiRes.error }
            }
          />
        </Card>
      </div>
    </div>
  );
}

function HealthRow({ label, ok, detail }: { label: string; ok: boolean; detail?: string }) {
  return (
    <div className="flex items-center justify-between rounded-lg border border-slate-800/70 bg-slate-950/50 px-3 py-2">
      <div>
        <p className="text-xs uppercase tracking-wide text-slate-500">{label}</p>
        {detail && <p className="text-xs text-slate-400">{detail}</p>}
      </div>
      <StatusPill status={ok ? "ok" : "error"} />
    </div>
  );
}

function KeyValue({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-slate-400">{label}</span>
      <span className="font-semibold text-slate-100">{value}</span>
    </div>
  );
}
