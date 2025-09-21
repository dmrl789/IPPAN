import { useState, useEffect, useMemo } from 'react'
import { Card, Button, Input, Badge, Switch, Select, SelectTrigger, SelectValue, SelectContent, SelectItem, Sheet, SheetContent, SheetHeader, SheetTitle, SheetFooter, Label } from '../components/UI'

/* ======================= Types ======================= */
type NetworkStats = { activeValidators: number; totalStaked: number; apy: number; blockTime: number };
type ValidatorStatus = "Active" | "Inactive" | "Jailed";
type Validator = {
  id: string;
  address: string;
  moniker: string;
  commission: number;     // %
  uptimePct: number;      // %
  votingPower: number;    // IPN or %
  selfStake: number;      // IPN
  status: ValidatorStatus;
  apr: number;            // % simple
  rank: number;
  location?: string;
  website?: string;
  produced24h: number;
  missed24h: number;
  slashed?: boolean;
};
type Delegation = { validatorId: string; validatorMoniker: string; amount: number; rewards: number; autoCompound: boolean };
type Unbonding = { validatorId: string; validatorMoniker: string; amount: number; requestedAt: string; completesAt: string };
type MyStaking = { totalDelegated: number; available: number; rewardsTotal: number; delegations: Delegation[]; unbondings: Unbonding[] };
type Estimation = { fee: number; etaSeconds: number; expectedMonthly: number };

type OperatorConfig = { isOperator: boolean; commission: number; moniker?: string; website?: string; endpoint?: string; logoUrl?: string };

/* =================== Local Storage Keys =================== */
const LS_ADDR = "ippan.wallet.address";

/* ======================= Mock APIs ======================= */
function nowMinus(hours: number) { return new Date(Date.now() - hours*3600e3).toLocaleString(); }
function inHours(h: number) { return new Date(Date.now() + h*3600e3).toLocaleString(); }
function rid(p="id") { return `${p}-${Math.random().toString(16).slice(2,10)}`; }

async function apiNetworkStats(): Promise<NetworkStats> {
  return new Promise(r => setTimeout(() => r({ activeValidators: 1234, totalStaked: 45_200_000, apy: 12.5, blockTime: 2.3 }), 120));
}
async function apiListValidators(): Promise<Validator[]> {
  const rows: Validator[] = Array.from({ length: 50 }).map((_, i) => ({
    id: `val${i+1}`,
    address: "i" + (1000+i).toString(16).padEnd(39,"a").slice(0,39),
    moniker: `Validator ${i+1}`,
    commission: Math.round((Math.random()*10+2)*10)/10, // 2–12%
    uptimePct: Math.round((96 + Math.random()*4)*10)/10,
    votingPower: Math.round(100_000 + Math.random()*2_000_000),
    selfStake: Math.round(10_000 + Math.random()*200_000),
    status: i % 17 === 0 ? "Inactive" : "Active",
    apr: Math.round((10 + Math.random()*5)*10)/10,
    rank: i+1,
    location: ["EU","US","APAC"][i%3],
    website: "https://example.com",
    produced24h: Math.floor(500 + Math.random()*1500),
    missed24h: Math.floor(Math.random()*20),
    slashed: Math.random() < 0.04,
  }));
  return new Promise(r => setTimeout(() => r(rows), 180));
}
async function apiMyStaking(_addr: string): Promise<MyStaking> {
  const delegations: Delegation[] = [
    { validatorId:"val1", validatorMoniker:"Validator 1", amount: 900, rewards: 24.5, autoCompound: true },
    { validatorId:"val12", validatorMoniker:"Validator 12", amount: 600, rewards: 8.2, autoCompound: false },
  ];
  const unbondings: Unbonding[] = [
    { validatorId:"val7", validatorMoniker:"Validator 7", amount: 200, requestedAt: nowMinus(12), completesAt: inHours(36) }
  ];
  return new Promise(r => setTimeout(() => r({
    totalDelegated: delegations.reduce((s,d)=>s+d.amount,0),
    available: 1_500,
    rewardsTotal: delegations.reduce((s,d)=>s+d.rewards,0),
    delegations, unbondings
  }), 150));
}
async function apiEstimateStake(_addr: string, _valId: string, amount: number): Promise<Estimation> {
  const fee = Math.max(0.02, Math.min(1, amount*0.002));
  const expectedMonthly = amount * 0.125 / 12; // using network APY ~12.5%
  return new Promise(r => setTimeout(() => r({ fee: Number(fee.toFixed(4)), etaSeconds: 10, expectedMonthly: Number(expectedMonthly.toFixed(4)) }), 140));
}
async function apiDelegate(_addr: string, _valId: string, _amount: number) { return new Promise(r => setTimeout(() => r({ ok: true }), 300)); }
async function apiUndelegate(_addr: string, _valId: string, _amount: number) { return new Promise(r => setTimeout(() => r({ ok: true }), 300)); }
async function apiRedelegate(_addr: string, _fromVal: string, _toVal: string, _amount: number) { return new Promise(r => setTimeout(() => r({ ok: true }), 350)); }
async function apiClaimRewards(_addr: string, _valId?: string) { return new Promise(r => setTimeout(() => r({ ok: true, claimed: Math.round(Math.random()*50)/10 }), 260)); }
async function apiSetAutoCompound(_addr: string, _valId: string, enable: boolean) { return new Promise(r => setTimeout(() => r({ ok: true, enabled: enable }), 180)); }

async function apiOperatorConfig(addr: string): Promise<OperatorConfig> {
  const isOp = addr?.endsWith("aaaa"); // arbitrary mock
  return new Promise(r => setTimeout(() => r({ isOperator: !!isOp, commission: 5.0, moniker: "My Validator", website: "https://myval.example", endpoint: "node.eu.ippan:30333", logoUrl: "" }), 120));
}
async function apiSetCommission(_addr: string, _pct: number) { return new Promise(r => setTimeout(() => r({ ok: true }), 200)); }
async function apiUpdateMetadata(_addr: string, _meta: Partial<OperatorConfig>) { return new Promise(r => setTimeout(() => r({ ok: true }), 220)); }
async function apiWithdrawCommission(_addr: string) { return new Promise(r => setTimeout(() => r({ ok: true, amount: Math.round(Math.random()*10*100)/100 }), 240)); }

/* ======================= Utils ======================= */
function loadAddress(): string | null { return localStorage.getItem(LS_ADDR); }
function fmt(n: number) { return n.toLocaleString(); }
function fmtIPN(n: number) { return `${n.toLocaleString(undefined,{maximumFractionDigits:4})} IPN`; }
function pct(n: number) { return `${n.toFixed(2)}%`; }
function hoursUntil(iso: string) { const d = (new Date(iso).getTime()-Date.now())/3600e3; return Math.max(0, Math.round(d)); }

/* ======================= Component ======================= */
export default function StakingValidator() {
  // Wallet
  const [address] = useState<string | null>(loadAddress());

  // Network & lists
  const [net, setNet] = useState<NetworkStats | null>(null);
  const [vals, setVals] = useState<Validator[]>([]);
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<"all"|"Active"|"Inactive"|"Jailed">("all");
  const [maxCommission, setMaxCommission] = useState<number>(100);
  const [minUptime, setMinUptime] = useState<number>(0);
  const [sortBy, setSortBy] = useState<"rank"|"commission"|"uptime"|"apr"|"votingPower">("rank");

  // My staking
  const [mine, setMine] = useState<MyStaking | null>(null);

  // Actions: stake/unstake/redelegate
  const [stakeOpen, setStakeOpen] = useState(false);
  const [stakeVal, setStakeVal] = useState<string>("");
  const [stakeAmt, setStakeAmt] = useState<string>("");
  const [est, setEst] = useState<Estimation | null>(null);

  const [unstakeOpen, setUnstakeOpen] = useState(false);
  const [unstakeVal, setUnstakeVal] = useState<string>("");
  const [unstakeAmt, setUnstakeAmt] = useState<string>("");

  const [redelegateOpen, setRedelegateOpen] = useState(false);
  const [fromVal, setFromVal] = useState<string>(""); const [toVal, setToVal] = useState<string>(""); const [redeAmt, setRedeAmt] = useState<string>("");

  // Detail drawer
  const [detailOpen, setDetailOpen] = useState(false);
  const [detail, setDetail] = useState<Validator | null>(null);

  // Operator panel
  const [op, setOp] = useState<OperatorConfig | null>(null);
  const [opCommission, setOpCommission] = useState<number>(5);
  const [opMoniker, setOpMoniker] = useState<string>(""); const [opWebsite, setOpWebsite] = useState<string>(""); const [opEndpoint, setOpEndpoint] = useState<string>("");

  // Load
  useEffect(() => { 
    apiNetworkStats().then(setNet); 
    apiListValidators().then(setVals); 
    if (address) {
      apiMyStaking(address).then(setMine); 
      apiOperatorConfig(address).then(cfg=>{ 
        setOp(cfg); 
        setOpCommission(cfg.commission); 
        setOpMoniker(cfg.moniker||""); 
        setOpWebsite(cfg.website||""); 
        setOpEndpoint(cfg.endpoint||""); 
      }); 
    }
  }, [address]);

  // Estimation
  useEffect(() => {
    const amt = Number(stakeAmt||"0");
    if (!address || !stakeVal || !amt || amt<=0) { setEst(null); return; }
    let alive = true;
    apiEstimateStake(address, stakeVal, amt).then(e => { if (alive) setEst(e); });
    return () => { alive = false; };
  }, [address, stakeVal, stakeAmt]);

  // Derived lists
  const filtered = useMemo(() => {
    let r = vals;
    if (query) { const q = query.toLowerCase(); r = r.filter(v => v.moniker.toLowerCase().includes(q) || v.address.toLowerCase().includes(q) || v.id.toLowerCase().includes(q)); }
    if (statusFilter !== "all") r = r.filter(v => v.status === statusFilter);
    r = r.filter(v => v.commission <= maxCommission && v.uptimePct >= minUptime);
    r = [...r].sort((a,b) => {
      if (sortBy === "rank") return a.rank - b.rank;
      if (sortBy === "commission") return a.commission - b.commission;
      if (sortBy === "uptime") return b.uptimePct - a.uptimePct;
      if (sortBy === "apr") return b.apr - a.apr;
      if (sortBy === "votingPower") return b.votingPower - a.votingPower;
      return 0;
    });
    return r;
  }, [vals, query, statusFilter, maxCommission, minUptime, sortBy]);

  /* ============== Handlers ============== */
  async function doStake() {
    const amt = Number(stakeAmt||"0"); if (!address || !stakeVal || !amt || amt<=0) return alert("Enter validator and amount");
    const ok = (await apiDelegate(address, stakeVal, amt));
    if (ok) {
      setStakeOpen(false); setStakeAmt(""); setStakeVal("");
      if (address) setMine(await apiMyStaking(address));
    }
  }
  async function doUnstake() {
    const amt = Number(unstakeAmt||"0"); if (!address || !unstakeVal || !amt || amt<=0) return alert("Enter validator and amount");
    const ok = (await apiUndelegate(address, unstakeVal, amt));
    if (ok) {
      setUnstakeOpen(false); setUnstakeAmt(""); setUnstakeVal("");
      if (address) setMine(await apiMyStaking(address));
    }
  }
  async function doRedelegate() {
    const amt = Number(redeAmt||"0"); if (!address || !fromVal || !toVal || !amt || amt<=0) return alert("Enter from, to and amount");
    const ok = (await apiRedelegate(address, fromVal, toVal, amt));
    if (ok) {
      setRedelegateOpen(false); setRedeAmt(""); setFromVal(""); setToVal("");
      if (address) setMine(await apiMyStaking(address));
    }
  }
  async function claim(valId?: string) {
    if (!address) return;
    const r:any = await apiClaimRewards(address, valId);
    alert(`Claimed ~${r.claimed} IPN`);
    if (address) setMine(await apiMyStaking(address));
  }
  async function toggleCompound(valId: string, enable: boolean) {
    if (!address) return; const ok:any = await apiSetAutoCompound(address, valId, enable);
    if (ok) setMine(m => m ? { ...m, delegations: m.delegations.map(d => d.validatorId===valId ? { ...d, autoCompound: enable } : d) } : m);
  }

  async function saveOperator() {
    if (!op || !address) return;
    await apiSetCommission(address, opCommission);
    await apiUpdateMetadata(address, { moniker: opMoniker, website: opWebsite, endpoint: opEndpoint });
    alert("Operator settings saved.");
    setOp(prev => prev ? { ...prev, commission: opCommission, moniker: opMoniker, website: opWebsite, endpoint: opEndpoint } : prev);
  }
  async function withdrawCommission() {
    if (!op || !address) return;
    const r:any = await apiWithdrawCommission(address);
    alert(`Withdrawn ${r.amount} IPN in commission.`);
  }

  /* ========================= UI ========================= */
  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Staking & Validator</h1>
          <p className="text-sm text-gray-600">Stake IPN to secure the network and earn rewards. Manage delegations and validator settings.</p>
        </div>
        <div className="text-right">
          {address ? (
            <>
              <div className="text-xs text-gray-600">Connected</div>
              <div className="font-mono text-sm">{address}</div>
            </>
          ) : (
            <div className="text-sm text-gray-600">Connect a wallet to manage staking.</div>
          )}
        </div>
      </header>

      {/* Network KPIs */}
      <div className="grid md:grid-cols-4 gap-4">
        <Card title="🛡️ Active Validators">
          <div className="text-3xl font-semibold">{net ? fmt(net.activeValidators) : "—"}</div>
        </Card>
        <Card title="🏆 Total Staked">
          <div className="text-3xl font-semibold">{net ? fmt(net.totalStaked) : "—"} IPN</div>
        </Card>
        <Card title="📊 APY">
          <div className="text-3xl font-semibold">{net ? `${net.apy}%` : "—"}</div>
        </Card>
        <Card title="⏱️ Block Time">
          <div className="text-3xl font-semibold">{net ? `${net.blockTime}s` : "—"}</div>
        </Card>
      </div>

      {/* My Staking */}
      <Card title="📊 My Staking">
        <div className="flex items-center justify-between mb-4">
          <div className="text-sm text-gray-600">Manage your delegations and rewards</div>
          <div className="flex gap-2">
            <Button onClick={() => setStakeOpen(true)}>
              ⬆️ Stake
            </Button>
            <Button onClick={() => setUnstakeOpen(true)} className="bg-gray-600 hover:bg-gray-700">
              ⬇️ Unstake
            </Button>
            <Button onClick={() => setRedelegateOpen(true)} className="bg-gray-600 hover:bg-gray-700">
              🔄 Redelegate
            </Button>
            <Button onClick={()=>claim()} className="bg-gray-600 hover:bg-gray-700">
              💰 Claim All
            </Button>
          </div>
        </div>
        
        {!mine ? (
          <p className="text-sm text-gray-600">Connect your wallet to see delegations.</p>
        ) : (
          <div className="space-y-4">
            <div className="grid md:grid-cols-3 gap-4">
              <div className="rounded-xl border p-3">
                <div className="text-sm text-gray-600">Available</div>
                <div className="text-2xl font-semibold">{fmtIPN(mine.available)}</div>
              </div>
              <div className="rounded-xl border p-3">
                <div className="text-sm text-gray-600">Delegated</div>
                <div className="text-2xl font-semibold">{fmtIPN(mine.totalDelegated)}</div>
              </div>
              <div className="rounded-xl border p-3">
                <div className="text-sm text-gray-600">Unclaimed Rewards</div>
                <div className="text-2xl font-semibold">{fmtIPN(mine.rewardsTotal)}</div>
              </div>
            </div>

            {/* Delegations */}
            <div>
              <div className="text-sm font-medium mb-2">Delegations</div>
              {(mine.delegations||[]).length===0 ? <p className="text-sm text-gray-600">No delegations yet.</p> : (
                <div className="grid gap-2">
                  {mine.delegations.map(d => (
                    <div key={d.validatorId} className="rounded-xl border p-3">
                     <div className="flex items-center justify-between">
                       <div>
                         <div className="font-medium">{d.validatorMoniker}</div>
                         <div className="text-xs text-gray-600">Staked: {fmtIPN(d.amount)} • Rewards: {fmtIPN(d.rewards)}</div>
                       </div>
                       <div className="flex items-center gap-3">
                         <div className="flex items-center gap-2">
                           <Switch 
                             checked={d.autoCompound} 
                             onCheckedChange={(checked) => toggleCompound(d.validatorId, checked)} 
                           />
                           <span className="text-xs">Auto-compound</span>
                         </div>
                         <Button onClick={()=>claim(d.validatorId)} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1">Claim</Button>
                         <Button onClick={()=>{ setUnstakeVal(d.validatorId); setUnstakeAmt(""); setUnstakeOpen(true); }} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1">Unstake</Button>
                       </div>
                     </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Unbonding queue */}
            <div>
              <div className="text-sm font-medium mb-2">Unbonding</div>
              {(mine.unbondings||[]).length===0 ? <p className="text-sm text-gray-600">No pending unbonding.</p> : (
                <div className="grid gap-2">
                  {mine.unbondings.map(u => (
                    <div key={rid("unb")} className="rounded-xl border p-3 flex items-center justify-between">
                      <div>
                        <div className="font-medium">{u.validatorMoniker}</div>
                        <div className="text-xs text-gray-600">Requested: {u.requestedAt} • Completes in ~{hoursUntil(u.completesAt)}h</div>
                      </div>
                      <div className="font-medium">{fmtIPN(u.amount)}</div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}
      </Card>

      {/* Validator Directory */}
      <Card title="🌐 Validator Directory">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Input placeholder="Search moniker / address…" value={query} onChange={(e)=>setQuery(e.target.value)} className="w-64" />
            <Select value={sortBy} onValueChange={(value) => setSortBy(value as any)}>
              <option value="rank">Rank</option>
              <option value="commission">Commission</option>
              <option value="uptime">Uptime</option>
              <option value="apr">APR</option>
              <option value="votingPower">Voting power</option>
            </Select>
            <Select value={statusFilter} onValueChange={(value) => setStatusFilter(value as any)}>
              <option value="all">All</option>
              <option value="Active">Active</option>
              <option value="Inactive">Inactive</option>
              <option value="Jailed">Jailed</option>
            </Select>
          </div>
          <div className="flex items-center gap-2 text-xs text-gray-600">
            <span>Max commission: {maxCommission}%</span>
            <input type="range" min="0" max="100" value={maxCommission} onChange={(e)=>setMaxCommission(Number(e.target.value))} className="w-32" />
            <span>Min uptime: {minUptime}%</span>
            <input type="range" min="0" max="100" value={minUptime} onChange={(e)=>setMinUptime(Number(e.target.value))} className="w-32" />
          </div>
        </div>
        
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="text-left text-gray-600">
              <tr>
                <th className="py-2 pr-3">#</th>
                <th className="py-2 pr-3">Moniker</th>
                <th className="py-2 pr-3">Uptime</th>
                <th className="py-2 pr-3">Commission</th>
                <th className="py-2 pr-3">APR</th>
                <th className="py-2 pr-3">Voting Power</th>
                <th className="py-2 pr-3">Self-stake</th>
                <th className="py-2 pr-3">Status</th>
                <th className="py-2 pr-3">Actions</th>
              </tr>
            </thead>
            <tbody>
              {filtered.length===0 ? (
                <tr><td colSpan={9} className="py-3">No validators found.</td></tr>
              ) : filtered.map(v => (
                <tr key={v.id} className="border-t">
                  <td className="py-2 pr-3">#{v.rank}</td>
                  <td className="py-2 pr-3">
                    <div className="font-medium flex items-center gap-2">
                      {v.moniker} {v.slashed && <Badge variant="error">⚠️ Slashed</Badge>}
                    </div>
                    <div className="text-xs text-gray-600 font-mono">{v.address}</div>
                  </td>
                  <td className="py-2 pr-3">{v.uptimePct.toFixed(2)}%</td>
                  <td className="py-2 pr-3">{v.commission}%</td>
                  <td className="py-2 pr-3">{v.apr}%</td>
                  <td className="py-2 pr-3">{fmt(v.votingPower)}</td>
                  <td className="py-2 pr-3">{fmt(v.selfStake)}</td>
                  <td className="py-2 pr-3">
                    <Badge variant={v.status==="Active" ? "default" : v.status==="Inactive" ? "warning" : "error"}>{v.status}</Badge>
                  </td>
                  <td className="py-2 pr-3">
                    <div className="flex flex-wrap gap-2">
                      <Button onClick={()=>{ setStakeVal(v.id); setStakeAmt(""); setStakeOpen(true); }} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1">Delegate</Button>
                      <Button onClick={()=>{ setDetail(v); setDetailOpen(true); }} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1">Details</Button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      {/* Validator Detail Drawer */}
      <Sheet open={detailOpen} onOpenChange={setDetailOpen}>
        <SheetContent className="w-full sm:max-w-xl">
          <SheetHeader>
            <SheetTitle>Validator Details</SheetTitle>
          </SheetHeader>
          {!detail ? <div className="p-4 text-sm">No validator selected.</div> : (
            <div className="py-4 space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-lg font-semibold">{detail.moniker} • #{detail.rank}</div>
                  <div className="text-xs text-gray-600 font-mono">{detail.address}</div>
                </div>
                <Badge variant={detail.status==="Active" ? "default" : detail.status==="Inactive" ? "warning" : "error"}>{detail.status}</Badge>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div className="rounded-xl border p-3">
                  <div className="text-xs text-gray-600">Commission</div>
                  <div className="text-lg font-medium">{detail.commission}%</div>
                </div>
                <div className="rounded-xl border p-3">
                  <div className="text-xs text-gray-600">Uptime</div>
                  <div className="text-lg font-medium">{detail.uptimePct}%</div>
                </div>
                <div className="rounded-xl border p-3">
                  <div className="text-xs text-gray-600">APR</div>
                  <div className="text-lg font-medium">{detail.apr}%</div>
                </div>
                <div className="rounded-xl border p-3">
                  <div className="text-xs text-gray-600">Voting Power</div>
                  <div className="text-lg font-medium">{fmt(detail.votingPower)}</div>
                </div>
                <div className="rounded-xl border p-3">
                  <div className="text-xs text-gray-600">Produced (24h)</div>
                  <div className="text-lg font-medium">{detail.produced24h}</div>
                </div>
                <div className="rounded-xl border p-3">
                  <div className="text-xs text-gray-600">Missed (24h)</div>
                  <div className="text-lg font-medium">{detail.missed24h}</div>
                </div>
              </div>
              <div className="text-xs text-gray-600">
                {detail.website && <a className="inline-flex items-center gap-1 underline mr-3" href={detail.website}>🔗 Website</a>}
                {detail.location && <span className="inline-flex items-center gap-1">🌍 {detail.location}</span>}
              </div>
              <div className="flex gap-2">
                <Button onClick={()=>{ setStakeVal(detail.id); setStakeAmt(""); setStakeOpen(true); }}>Delegate</Button>
                <Button onClick={()=>{ setFromVal(detail.id); setToVal(""); setRedeAmt(""); setRedelegateOpen(true); }} className="bg-gray-600 hover:bg-gray-700">Redelegate from this</Button>
              </div>
              {detail.slashed && (
                <div className="rounded-xl border p-3 bg-yellow-50">
                  <div className="flex items-center gap-2 text-yellow-700">⚠️ Slashing Notice</div>
                  <div className="text-xs text-yellow-700 mt-1">This validator was slashed recently. Review their performance and commission before delegating.</div>
                </div>
              )}
            </div>
          )}
          <SheetFooter>
            <div></div>
          </SheetFooter>
        </SheetContent>
      </Sheet>

      {/* Operator Panel (if you are a validator) */}
      {op?.isOperator && (
        <Card title="⚙️ Operator Panel">
          <div className="space-y-4">
            <div className="grid md:grid-cols-3 gap-4">
              <div className="rounded-xl border p-3">
                <div className="text-sm font-medium mb-1">Commission Rate</div>
                <div className="flex items-center gap-3">
                  <input type="range" min="0" max="20" step="0.1" value={opCommission} onChange={(e)=>setOpCommission(Number(e.target.value))} className="w-48" />
                  <div className="text-sm">{opCommission.toFixed(1)}%</div>
                </div>
              </div>
              <div className="rounded-xl border p-3">
                <div className="text-sm font-medium mb-1">Moniker</div>
                <Input value={opMoniker} onChange={(e)=>setOpMoniker(e.target.value)} />
              </div>
              <div className="rounded-xl border p-3">
                <div className="text-sm font-medium mb-1">Website</div>
                <Input value={opWebsite} onChange={(e)=>setOpWebsite(e.target.value)} placeholder="https://…" />
              </div>
              <div className="rounded-xl border p-3 md:col-span-2">
                <div className="text-sm font-medium mb-1">Endpoint</div>
                <Input value={opEndpoint} onChange={(e)=>setOpEndpoint(e.target.value)} placeholder="node.host:port" />
              </div>
            </div>
            <div className="flex gap-2">
              <Button onClick={saveOperator}>✅ Save</Button>
              <Button onClick={withdrawCommission} className="bg-gray-600 hover:bg-gray-700">💰 Withdraw Commission</Button>
            </div>
            <div className="text-xs text-gray-600">
              Tip: keep downtime low and commission competitive. Provide a status page and contact for delegators.
            </div>
          </div>
        </Card>
      )}

      {/* Risk & Health */}
      <Card title="📊 Health & Risk">
        <div className="space-y-2">
          <div className="rounded-xl border p-3 bg-gray-50">
            <div className="text-sm font-medium">Best practices</div>
            <ul className="list-disc ml-5 text-sm text-gray-600">
              <li>Diversify across multiple validators to reduce correlated risk.</li>
              <li>Prefer validators with high uptime, low missed rounds, and transparent ops.</li>
              <li>Mind the unbonding period; funds are illiquid until completion.</li>
              <li>Auto-compound can boost APY but increases exposure; adjust per validator.</li>
            </ul>
          </div>
        </div>
      </Card>

      {/* Action Dialogs */}
      {/* Stake Dialog */}
      {stakeOpen && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-96">
            <h3 className="text-lg font-semibold mb-4">Stake Tokens</h3>
            <div className="space-y-3">
              <Label>Validator</Label>
              <Select value={stakeVal} onValueChange={(value) => setStakeVal(value)}>
                <option value="">Choose validator</option>
                {vals.map(v => <option key={v.id} value={v.id}>{v.moniker} • #{v.rank} • {v.commission}%</option>)}
              </Select>
              <Label>Amount to Stake (IPN)</Label>
              <Input placeholder="0.00" value={stakeAmt} onChange={(e)=>setStakeAmt(e.target.value)} />
              <div className="text-xs text-gray-600">
                Fee: {est ? `${est.fee} IPN` : "—"} • ETA: {est ? `${est.etaSeconds}s` : "—"} • Est. monthly rewards: {est ? `${est.expectedMonthly} IPN` : "—"}
              </div>
            </div>
            <div className="flex gap-2 mt-4">
              <Button onClick={()=>setStakeOpen(false)} className="bg-gray-600 hover:bg-gray-700">Cancel</Button>
              <Button onClick={doStake}>Stake</Button>
            </div>
          </div>
        </div>
      )}

      {/* Unstake Dialog */}
      {unstakeOpen && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-96">
            <h3 className="text-lg font-semibold mb-4">Unstake Tokens</h3>
            <div className="space-y-3">
              <Label>From Validator</Label>
              <Select value={unstakeVal} onValueChange={(value) => setUnstakeVal(value)}>
                <option value="">Choose validator</option>
                {(mine?.delegations||[]).map(d => <option key={d.validatorId} value={d.validatorId}>{d.validatorMoniker}</option>)}
              </Select>
              <Label>Amount to Unstake (IPN)</Label>
              <Input placeholder="0.00" value={unstakeAmt} onChange={(e)=>setUnstakeAmt(e.target.value)} />
              <div className="text-xs text-gray-600">Unbonding takes a protocol-defined period before funds are liquid.</div>
            </div>
            <div className="flex gap-2 mt-4">
              <Button onClick={()=>setUnstakeOpen(false)} className="bg-gray-600 hover:bg-gray-700">Cancel</Button>
              <Button onClick={doUnstake}>Unstake</Button>
            </div>
          </div>
        </div>
      )}

      {/* Redelegate Dialog */}
      {redelegateOpen && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-96">
            <h3 className="text-lg font-semibold mb-4">Redelegate</h3>
            <div className="space-y-3">
              <Label>From</Label>
              <Select value={fromVal} onValueChange={(value) => setFromVal(value)}>
                <option value="">Validator you staked to</option>
                {(mine?.delegations||[]).map(d => <option key={d.validatorId} value={d.validatorId}>{d.validatorMoniker}</option>)}
              </Select>
              <Label>To</Label>
              <Select value={toVal} onValueChange={(value) => setToVal(value)}>
                <option value="">New validator</option>
                {vals.map(v => <option key={v.id} value={v.id}>{v.moniker} • {v.commission}%</option>)}
              </Select>
              <Label>Amount (IPN)</Label>
              <Input placeholder="0.00" value={redeAmt} onChange={(e)=>setRedeAmt(e.target.value)} />
            </div>
            <div className="flex gap-2 mt-4">
              <Button onClick={()=>setRedelegateOpen(false)} className="bg-gray-600 hover:bg-gray-700">Cancel</Button>
              <Button onClick={doRedelegate}>Redelegate</Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
