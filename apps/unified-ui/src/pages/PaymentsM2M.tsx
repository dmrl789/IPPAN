import { useState, useEffect, useMemo } from 'react'
import { Card, Button, Input, Badge, Switch, Select, SelectTrigger, SelectValue, SelectContent, SelectItem, Sheet, SheetContent, SheetHeader, SheetTitle, SheetFooter, Label, Textarea } from '../components/UI'

// =============== Types ===============
type FeePreview = { fee: number; nonce: number; etaSeconds: number };
type ChannelState = "Active" | "Pending" | "Paused" | "Closing" | "Closed";
type Channel = {
  id: string;
  peer: string;
  state: ChannelState;
  capacity: number;     // total locked
  localBalance: number; // ours
  remoteBalance: number;// peer
  openedAt: string;
};
type Stream = {
  id: string;
  channelId: string;
  peer: string;
  rate: number; // IPN/sec
  startedAt: string;
  paused: boolean;
};
type Meter = {
  id: string;
  name: string;        // e.g. "API calls"
  unit: string;        // "call", "MB", "request"
  pricePerUnit: number;// IPN per unit
  description?: string;
};
type Device = {
  id: string;
  name: string;
  apiKey: string;
  scopes: string[]; // allowed meters
  monthlyCap: number; // IPN cap
  enabled: boolean;
};
type Invoice = {
  id: string;
  to: string;
  amount: number;
  memo?: string;
  status: "Pending" | "Paid" | "Expired";
  link: string;
  createdAt: string;
  expiresAt: string;
};
type Activity = { id: string; title: string; subtitle: string; delta: number; when: string; status?: string };

// =============== Mock APIs (swap for real) ===============
function nowMinus(hours: number) { return new Date(Date.now() - hours*3600e3).toLocaleString(); }
function rid(prefix="id") { return `${prefix}-${Math.random().toString(16).slice(2,10)}`; }

async function apiEstimateFee(_from: string, _to: string, amount: number): Promise<FeePreview> {
  const base = Math.max(0.01, Math.min(1, amount*0.002));
  return new Promise(r=>setTimeout(()=>r({ fee: Number(base.toFixed(4)), nonce: Math.floor(Math.random()*1000), etaSeconds: 5 }),180));
}
async function apiSendPayment(_from: string, _to: string, _amount: number, _memo?: string): Promise<{ok:boolean}> {
  return new Promise(r=>setTimeout(()=>r({ok:true}),350));
}

async function apiListChannels(): Promise<Channel[]> {
  return new Promise(r=>setTimeout(()=>r([
    { id: "ch1", peer: "iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X", state:"Active", capacity: 300, localBalance:150, remoteBalance:150, openedAt: nowMinus(72) },
          { id: "ch2", peer: "iDLZ4d490pJHpsL2PDoXTDA8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0X", state:"Pending", capacity: 100, localBalance:75, remoteBalance:25, openedAt: nowMinus(24*10) },
          { id: "ch3", peer: "iEBSQkH2jVt5B1jq2vMmO3b7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0f9e8X", state:"Active", capacity: 220, localBalance:90, remoteBalance:130, openedAt: nowMinus(24*2) }
  ]),160));
}
async function apiOpenChannel(peer: string, deposit: number, challengeHours: number): Promise<{ok:boolean; channel?:Channel}> {
  const ch: Channel = { id: rid("ch"), peer, state:"Pending", capacity: deposit, localBalance: deposit, remoteBalance: 0, openedAt: new Date().toLocaleString() };
  console.log("OPEN CHANNEL", { peer, deposit, challengeHours });
  return new Promise(r=>setTimeout(()=>r({ok:true, channel: ch}),500));
}
async function apiTopUpChannel(id: string, amount: number): Promise<{ok:boolean}> { console.log("TOPUP", id, amount); return new Promise(r=>setTimeout(()=>r({ok:true}),240)); }
async function apiPauseChannel(id: string, pause: boolean): Promise<{ok:boolean; state:ChannelState}> { return new Promise(r=>setTimeout(()=>r({ok:true, state: pause ? "Paused":"Active"}),180)); }
async function apiCloseChannel(id: string): Promise<{ok:boolean; state:ChannelState}> { return new Promise(r=>setTimeout(()=>r({ok:true, state:"Closing"}),220)); }
async function apiSettleChannel(id: string): Promise<{ok:boolean; state:ChannelState}> { return new Promise(r=>setTimeout(()=>r({ok:true, state:"Closed"}),500)); }
async function apiSendMicro(id: string, amount: number): Promise<{ok:boolean}> { console.log("MICRO", id, amount); return new Promise(r=>setTimeout(()=>r({ok:true}),120)); }

async function apiListStreams(): Promise<Stream[]> {
  return new Promise(r=>setTimeout(()=>r([
    { id:"st1", channelId:"ch1", peer:"iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X", rate:0.02, startedAt: nowMinus(1), paused:false }
  ]),120));
}
async function apiStartStream(channelId: string, rate: number): Promise<{ok:boolean; stream:Stream}> {
  const s: Stream = { id: rid("st"), channelId, peer:"peer?", rate, startedAt: new Date().toLocaleString(), paused:false };
  return new Promise(r=>setTimeout(()=>r({ok:true, stream:s}),220));
}
async function apiStopStream(id: string): Promise<{ok:boolean}> { return new Promise(r=>setTimeout(()=>r({ok:true}),200)); }

async function apiListMeters(): Promise<Meter[]> {
  return new Promise(r=>setTimeout(()=>r([
    { id:"m1", name:"API calls", unit:"call", pricePerUnit:0.001, description:"Per request" },
    { id:"m2", name:"Bandwidth", unit:"MB", pricePerUnit:0.0002, description:"Per MB egress" }
  ]),120));
}
async function apiCreateMeter(name: string, unit: string, pricePerUnit: number, description?: string): Promise<{ok:boolean; meter:Meter}> {
  const m: Meter = { id: rid("m"), name, unit, pricePerUnit, description };
  return new Promise(r=>setTimeout(()=>r({ok:true, meter:m}),200));
}
async function apiRecordUsage(meterId: string, units: number, channelId: string): Promise<{ok:boolean; charged:number}> {
  const charged = Number((units * 0.001).toFixed(6));
  console.log("USAGE", { meterId, units, channelId, charged });
  return new Promise(r=>setTimeout(()=>r({ok:true, charged}),160));
}

async function apiListDevices(): Promise<Device[]> {
  return new Promise(r=>setTimeout(()=>r([
    { id:"dv1", name:"sensor-01", apiKey:"sk_live_"+Math.random().toString(36).slice(2,10), scopes:["m1","m2"], monthlyCap:100, enabled:true },
    { id:"dv2", name:"gateway-eu", apiKey:"sk_live_"+Math.random().toString(36).slice(2,10), scopes:["m1"], monthlyCap:50, enabled:true }
  ]),120));
}
async function apiCreateDevice(name: string, scopes: string[], cap: number): Promise<{ok:boolean; device:Device}> {
  const d: Device = { id: rid("dv"), name, apiKey:"sk_live_"+Math.random().toString(36).slice(2,10), scopes, monthlyCap:cap, enabled:true };
  return new Promise(r=>setTimeout(()=>r({ok:true, device:d}),200));
}
async function apiRotateKey(id: string): Promise<{ok:boolean; apiKey:string}> {
  return new Promise(r=>setTimeout(()=>r({ok:true, apiKey:"sk_live_"+Math.random().toString(36).slice(2,10)}),200));
}
async function apiToggleDevice(id: string, enabled: boolean): Promise<{ok:boolean}> { return new Promise(r=>setTimeout(()=>r({ok:true}),140)); }

async function apiListInvoices(): Promise<Invoice[]> {
  return new Promise(r=>setTimeout(()=>r([
    { id:"inv1", to:"iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X", amount:25.5, memo:"Lunch payment", status:"Paid", link:"https://pay.ippan/inv1", createdAt: nowMinus(4), expiresAt: nowMinus(-24) },
          { id:"inv2", to:"iDLZ4d490pJHpsL2PDoXTDA8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0X", amount:500, memo:"Project payment", status:"Pending", link:"https://pay.ippan/inv2", createdAt: nowMinus(26), expiresAt: nowMinus(-1) },
  ]),160));
}
async function apiCreateInvoice(to: string, amount: number, memo?: string, ttlMins=60): Promise<{ok:boolean; invoice:Invoice}> {
  const inv: Invoice = {
    id: rid("inv"), to, amount, memo, status:"Pending",
    link: `https://pay.ippan/${Math.random().toString(36).slice(2,10)}`,
    createdAt: new Date().toLocaleString(),
    expiresAt: new Date(Date.now()+ttlMins*60e3).toLocaleString()
  };
  return new Promise(r=>setTimeout(()=>r({ok:true, invoice: inv}),220));
}
async function apiWebhookConfig(): Promise<{url:string; secret:string; events:string[]}> {
  return new Promise(r=>setTimeout(()=>r({ url:"https://example.com/webhooks/ippan", secret:"whsec_"+Math.random().toString(36).slice(2,12), events:["channel.opened","channel.updated","invoice.paid","stream.started","stream.stopped"] }),120));
}
async function apiSaveWebhook(url: string, events: string[]): Promise<{ok:boolean; secret?:string}> {
  return new Promise(r=>setTimeout(()=>r({ok:true, secret:"whsec_"+Math.random().toString(36).slice(2,12)}),200));
}

// =============== Component ===============
interface PaymentsM2MProps {
  walletAddress: string | null;
  walletConnected: boolean;
}

export default function PaymentsM2M({ walletAddress, walletConnected }: PaymentsM2MProps) {
  // Use wallet state from props instead of local state
  const address = walletAddress;
  const connected = walletConnected;

  // Pay form
  const [to, setTo] = useState("");
  const [amount, setAmount] = useState("");
  const [memo, setMemo] = useState("");
  const [fee, setFee] = useState<FeePreview | null>(null);

  // Channels
  const [channels, setChannels] = useState<Channel[]>([]);
  const [streams, setStreams] = useState<Stream[]>([]);
  const [openPeer, setOpenPeer] = useState("");
  const [openDeposit, setOpenDeposit] = useState("100");
  const [openChallenge, setOpenChallenge] = useState("24");
  const [openSheet, setOpenSheet] = useState(false);

  // Meters & Devices
  const [meters, setMeters] = useState<Meter[]>([]);
  const [devices, setDevices] = useState<Device[]>([]);
  const [newMeter, setNewMeter] = useState({ name:"", unit:"call", ppu:"0.001", desc:"" });
  const [newDevice, setNewDevice] = useState({ name:"", scopes:[] as string[], cap:"100" });
  const [usage, setUsage] = useState({ meterId:"", units:"", channelId:"" });

  // Invoices & webhooks
  const [invoices, setInvoices] = useState<Invoice[]>([]);
  const [invTo, setInvTo] = useState(""); const [invAmt, setInvAmt] = useState(""); const [invMemo, setInvMemo] = useState("");
  const [wh, setWh] = useState<{url:string; secret:string; events:string[]}|null>(null);

  // Activity + KPIs
  const [activity, setActivity] = useState<Activity[]>([
    { id: "a1", title:"Payment Sent", subtitle:"To: iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X • Lunch payment", delta:-25.50, when: nowMinus(2), status:"Confirmed" },
          { id: "a2", title:"Payment Received", subtitle:"From: iDLZ4d490pJHpsL2PDoXTDA8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0X • Project payment", delta:+500.00, when: nowMinus(24), status:"Confirmed" },
      { id: "a3", title:"M2M Channel Opened", subtitle:"With: iEBSQkH2jVt5B1jq2vMmO3b7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0f9e8X • 100 IPN", delta:0, when: nowMinus(24*3), status:"Active" },
  ]);

  // Load data
  useEffect(()=>{ apiListChannels().then(setChannels); apiListStreams().then(setStreams); apiListMeters().then(setMeters); apiListDevices().then(setDevices); apiListInvoices().then(setInvoices); apiWebhookConfig().then(setWh); },[]);

  // Fee preview
  useEffect(()=>{
    const amt = Number(amount);
    if (!address || !to || !amt || amt<=0) { setFee(null); return; }
    let alive = true;
    apiEstimateFee(address, to, amt).then(f=>{ if (alive) setFee(f); });
    return ()=>{ alive=false; };
  },[address,to,amount]);

  const monthSent = useMemo(()=> activity.filter(a=>a.delta<0).reduce((s,a)=>s+Math.abs(a.delta),0),[activity]);
  const monthRecv = useMemo(()=> activity.filter(a=>a.delta>0).reduce((s,a)=>s+a.delta,0),[activity]);
  const activeChannels = useMemo(()=> channels.filter(c=>c.state==="Active").length,[channels]);

  // ----- Actions -----
  async function sendPayment() {
    if (!address) return alert("Connect wallet first.");
    const amt = Number(amount); if (!/^i[A-Za-z0-9]{34}$/.test(to) || !amt || amt<=0) return alert("Enter valid recipient and amount.");
    const ok = (await apiSendPayment(address,to,amt,memo)).ok;
    if (ok) {
      setActivity(a=>[{ id:rid("act"), title:"Payment Sent", subtitle:`To: ${to} • ${memo||"—"}`, delta:-amt, when:new Date().toLocaleString(), status:"Pending" }, ...a]);
      setTo(""); setAmount(""); setMemo("");
    }
  }

  async function openChannel() {
    const dep = Number(openDeposit); const chal = Number(openChallenge);
    if (!/^i[A-Za-z0-9]{34}$/.test(openPeer) || !dep || dep<=0) return alert("Enter peer and valid deposit.");
    const res = await apiOpenChannel(openPeer, dep, chal);
    if (res.ok && res.channel) {
      setChannels(prev=>[res.channel!, ...prev]);
      setOpenSheet(false);
    }
  }
  async function topUpChannel(id: string) {
    const amt = Number(prompt("Top-up amount (IPN):","50")||"0"); if (!amt || amt<=0) return;
    const ok = (await apiTopUpChannel(id, amt)).ok; if (!ok) return;
    setChannels(prev=>prev.map(c=> c.id===id ? { ...c, capacity:c.capacity+amt, localBalance:c.localBalance+amt } : c));
  }
  async function pauseResume(id: string, pause: boolean) {
    const res = await apiPauseChannel(id, pause);
    if (res.ok) setChannels(prev=>prev.map(c=> c.id===id ? { ...c, state:res.state } : c));
  }
  async function close(id: string) {
    const res = await apiCloseChannel(id); if (res.ok) setChannels(prev=>prev.map(c=> c.id===id ? { ...c, state:res.state } : c));
  }
  async function settle(id: string) {
    const res = await apiSettleChannel(id); if (res.ok) setChannels(prev=>prev.map(c=> c.id===id ? { ...c, state:res.state } : c));
  }
  async function micro(id: string) {
    const amt = Number(prompt("Micropayment amount (IPN):","0.05")||"0"); if (!amt || amt<=0) return;
    const ok = (await apiSendMicro(id, amt)).ok; if (ok) {
      setChannels(prev=>prev.map(c=> c.id===id ? { ...c, localBalance: Math.max(0, c.localBalance-amt), remoteBalance: c.remoteBalance+amt } : c));
    }
  }

  async function startStream(channelId: string) {
    const rate = Number(prompt("Rate (IPN/sec):","0.02")||"0"); if (!rate || rate<=0) return;
    const r = await apiStartStream(channelId, rate);
    if (r.ok) setStreams(prev=>[r.stream, ...prev]);
  }
  async function stopStream(streamId: string) {
    const ok = (await apiStopStream(streamId)).ok; if (ok) setStreams(prev=>prev.filter(s=>s.id!==streamId));
  }

  async function createMeter() {
    const { name, unit, ppu, desc } = newMeter;
    if (!name || !unit || !ppu) return;
    const r = await apiCreateMeter(name, unit, Number(ppu), desc||undefined);
    if (r.ok) { setMeters(prev=>[r.meter, ...prev]); setNewMeter({ name:"", unit:"call", ppu:"0.001", desc:"" }); }
  }
  async function createDevice() {
    if (!newDevice.name || newDevice.scopes.length===0) return alert("Name and at least one scope required.");
    const r = await apiCreateDevice(newDevice.name, newDevice.scopes, Number(newDevice.cap||"0"));
    if (r.ok) setDevices(prev=>[r.device, ...prev]);
  }
  async function rotateKey(id: string) {
    const r = await apiRotateKey(id); if (r.ok) setDevices(prev=>prev.map(d=> d.id===id ? { ...d, apiKey:r.apiKey } : d));
  }
  async function toggleDevice(id: string, enabled: boolean) {
    const r = await apiToggleDevice(id, enabled); if (r.ok) setDevices(prev=>prev.map(d=> d.id===id ? { ...d, enabled } : d));
  }

  async function recordUsage() {
    const u = Number(usage.units||"0"); if (!usage.meterId || !usage.channelId || !u) return;
    const r = await apiRecordUsage(usage.meterId, u, usage.channelId);
    if (r.ok) alert(`Charged ~${r.charged} IPN to channel ${usage.channelId}`);
    setUsage({ meterId:"", units:"", channelId:"" });
  }

  async function createInvoice() {
    const amt = Number(invAmt||"0"); if (!/^i[A-Za-z0-9]{34}$/.test(invTo) || !amt) return alert("Recipient and amount required.");
    const r = await apiCreateInvoice(invTo, amt, invMemo||undefined, 60);
    if (r.ok) setInvoices(prev=>[r.invoice, ...prev]);
    setInvTo(""); setInvAmt(""); setInvMemo("");
  }

  async function saveWebhook() {
    if (!wh) return;
    const ok = (await apiSaveWebhook(wh.url, wh.events)).ok;
    if (ok) alert("Webhook saved. Use the shared secret to verify signatures.");
  }

  // =============== UI ===============
  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">💳 Payments & M2M</h1>
          <p className="text-sm text-gray-600">{connected ? "Connected" : "Disconnected"}</p>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-gray-600">💼</span>
          <span className="font-mono text-sm">{address ?? "—"}</span>
          <span className={`px-2 py-1 rounded text-xs ${connected ? "bg-green-100 text-green-800" : "bg-red-100 text-red-800"}`}>
            {connected ? "Connected" : "Disconnected"}
          </span>
        </div>
      </header>

      {/* KPIs */}
      <div className="grid md:grid-cols-3 gap-4">
        <Card title="💰 This Month">
          <div className="flex gap-8">
            <div><div className="text-xs text-gray-600">Received</div><div className="text-xl font-semibold">+{monthRecv.toLocaleString()} IPN</div></div>
            <div><div className="text-xs text-gray-600">Sent</div><div className="text-xl font-semibold">-{monthSent.toLocaleString()} IPN</div></div>
          </div>
        </Card>
        <Card title="🔗 Active Channels">
          <div className="text-3xl font-semibold">{activeChannels}</div>
        </Card>
        <Card title="📊 Streams">
          <div className="text-3xl font-semibold">{streams.length}</div>
        </Card>
      </div>

             {/* Pay + Channels */}
       <div className="grid md:grid-cols-3 gap-4">
         {/* Send Payment */}
         <div className="md:col-span-1">
           <Card title="💳 Send Payment">
             <div className="space-y-4">
               <div className="space-y-2">
                 <Label className="text-sm font-medium text-gray-700">Recipient Address</Label>
                 <Input 
                   value={to} 
                   onChange={e=>setTo(e.target.value)} 
                   placeholder="i..." 
                   className="w-full"
                 />
               </div>
               
               <div className="space-y-2">
                 <Label className="text-sm font-medium text-gray-700">Amount (IPN)</Label>
                 <Input 
                   value={amount} 
                   onChange={e=>setAmount(e.target.value)} 
                   placeholder="0.00" 
                   className="w-full"
                 />
               </div>
               
               <div className="space-y-2">
                 <Label className="text-sm font-medium text-gray-700">Memo (Optional)</Label>
                 <Input 
                   value={memo} 
                   onChange={e=>setMemo(e.target.value)} 
                   placeholder="Payment description" 
                   className="w-full"
                 />
               </div>
               
               <div className="text-xs text-gray-600 bg-gray-50 p-2 rounded border">
                 Fee: {fee ? `${fee.fee} IPN` : "—"} • Nonce: {fee ? fee.nonce : "—"} • ETA: {fee ? `${fee.etaSeconds}s` : "—"}
               </div>
               
               <Button onClick={sendPayment} className="w-full">
                 📤 Send Payment
               </Button>
             </div>
           </Card>
         </div>

         {/* Channels */}
         <div className="md:col-span-2">
           <Card title="🔗 M2M Payment Channels">
             <div className="flex items-center justify-between mb-4">
               <div className="text-xs text-gray-600">State channels for off-chain micropayments</div>
               <Button onClick={() => setOpenSheet(true)}>
                 ➕ Create New Channel
               </Button>
             </div>
             <div className="space-y-3">
               {channels.length===0 ? <p className="text-sm text-gray-600">No channels yet.</p> : channels.map(c=>(
                 <div key={c.id} className="rounded-xl border p-4">
                   <div className="flex items-start justify-between mb-3">
                     <div className="flex-1 min-w-0 mr-3">
                       <div className="font-medium text-gray-900 break-all">With: {c.peer}</div>
                       <div className="text-xs text-gray-600 mt-1">Opened: {c.openedAt}</div>
                     </div>
                     <div className="flex-shrink-0">
                       <Badge variant={c.state==="Active" ? "default" : c.state==="Paused" ? "warning" : "error"}>{c.state}</Badge>
                     </div>
                   </div>
                   <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-3 text-sm">
                     <div className="text-gray-700">Capacity: <strong className="text-gray-900">{c.capacity} IPN</strong></div>
                     <div className="text-gray-700">Local: <strong className="text-gray-900">{c.localBalance} IPN</strong></div>
                     <div className="text-gray-700">Remote: <strong className="text-gray-900">{c.remoteBalance} IPN</strong></div>
                     <div className="text-gray-700">Utilization: <strong className="text-gray-900">{Math.round(((c.capacity-c.localBalance)/c.capacity)*100)}%</strong></div>
                   </div>
                   <div className="flex gap-1 overflow-x-auto pb-2">
                     <Button onClick={()=>topUpChannel(c.id)} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1 whitespace-nowrap">
                       Top-up
                     </Button>
                     <Button onClick={()=>micro(c.id)} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1 whitespace-nowrap">
                       ⚡ Micropay
                     </Button>
                     <Button onClick={()=>pauseResume(c.id, c.state!=="Paused")} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1 whitespace-nowrap">
                       {c.state==="Paused" ? "▶️ Resume" : "⏸️ Pause"}
                     </Button>
                     <Button onClick={()=>startStream(c.id)} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1 whitespace-nowrap">
                       📡 Start Stream
                     </Button>
                     <Button onClick={()=>close(c.id)} className="bg-red-600 hover:bg-red-700 text-xs px-2 py-1 whitespace-nowrap">
                       ❌ Close
                     </Button>
                     <Button onClick={()=>settle(c.id)} className="bg-green-600 hover:bg-green-700 text-xs px-2 py-1 whitespace-nowrap">
                       ✅ Settle
                     </Button>
                   </div>
                 </div>
               ))}
             </div>
           </Card>
         </div>
       </div>

      {/* Streams + Meters/Devices */}
      <div className="grid md:grid-cols-2 gap-4">
                 {/* Streams */}
         <Card title="📡 Streaming Payments">
           <div className="space-y-2">
             {streams.length===0 ? <p className="text-sm text-gray-600">No active streams.</p> :
               streams.map(s=>(
                 <div key={s.id} className="rounded-xl border p-3 flex items-start justify-between">
                   <div className="flex-1 min-w-0 mr-3">
                     <div className="font-medium break-all">Channel: {s.channelId} • {s.peer}</div>
                     <div className="text-xs text-gray-600 mt-1">Rate: {s.rate} IPN/sec • Since: {s.startedAt}</div>
                   </div>
                   <div className="flex-shrink-0">
                     <Button onClick={()=>stopStream(s.id)} className="bg-red-600 hover:bg-red-700 text-xs px-2 py-1 whitespace-nowrap">
                       Stop
                     </Button>
                   </div>
                 </div>
               ))
             }
             <p className="text-xs text-gray-600">Streams accrue continuously via off-chain updates; settle to finalize.</p>
           </div>
         </Card>

        {/* Meters & Devices */}
        <Card title="📊 Meters & Tariffs">
          <div className="space-y-4">
            <div className="rounded-xl border p-3">
              <div className="text-sm font-medium mb-2">Create Meter</div>
              <div className="grid md:grid-cols-4 gap-2">
                <Input placeholder="Name (e.g., API calls)" value={newMeter.name} onChange={e=>setNewMeter(m=>({...m, name:e.target.value}))} />
                <Input placeholder="Unit (call/MB/...)" value={newMeter.unit} onChange={e=>setNewMeter(m=>({...m, unit:e.target.value}))} />
                <Input placeholder="Price per unit (IPN)" value={newMeter.ppu} onChange={e=>setNewMeter(m=>({...m, ppu:e.target.value}))} />
                <Input placeholder="Description (optional)" value={newMeter.desc} onChange={e=>setNewMeter(m=>({...m, desc:e.target.value}))} />
              </div>
              <div className="mt-2"><Button onClick={createMeter} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1">➕ Add Meter</Button></div>
            </div>

            {meters.length===0 ? <p className="text-sm text-gray-600">No meters yet.</p> :
              <div className="grid gap-2">
                {meters.map(m=>(
                  <div key={m.id} className="rounded-xl border p-3 flex items-center justify-between">
                    <div>
                      <div className="font-medium">{m.name} • {m.unit}</div>
                      <div className="text-xs text-gray-600">{m.description || "—"}</div>
                    </div>
                    <div className="text-sm">{m.pricePerUnit} IPN/{m.unit}</div>
                  </div>
                ))}
              </div>
            }
          </div>
        </Card>
      </div>

      {/* Device Registry + Usage charge */}
      <Card title="🔧 Device Registry (API Keys)">
        <div className="text-xs text-gray-600 mb-4">Attach devices to meters; usage is charged to a channel.</div>
        <div className="space-y-4">
          <div className="rounded-xl border p-3">
            <div className="text-sm font-medium mb-2">Register Device</div>
            <div className="grid md:grid-cols-4 gap-2">
              <Input placeholder="Device name" value={newDevice.name} onChange={e=>setNewDevice(d=>({...d, name:e.target.value}))} />
              <Select value={newDevice.scopes[0]||""} onValueChange={(v)=>setNewDevice(d=>({...d, scopes: v ? [v] : []}))}>
                <SelectTrigger><SelectValue placeholder="Scope (meter)" /></SelectTrigger>
                <SelectContent>
                  {meters.map(m=><SelectItem key={m.id} value={m.id}>{m.name}</SelectItem>)}
                </SelectContent>
              </Select>
              <Input placeholder="Monthly cap (IPN)" value={newDevice.cap} onChange={e=>setNewDevice(d=>({...d, cap:e.target.value}))} />
              <Button onClick={createDevice} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1">Create</Button>
            </div>
          </div>

          {devices.length===0 ? <p className="text-sm text-gray-600">No devices.</p> :
            <div className="grid gap-2">
              {devices.map(d=>(
                <div key={d.id} className="rounded-xl border p-3">
                  <div className="flex items-center justify-between">
                    <div>
                      <div className="font-medium">{d.name}</div>
                      <div className="text-xs text-gray-600">Scopes: {d.scopes.join(", ") || "—"} • Cap: {d.monthlyCap} IPN</div>
                    </div>
                    <div className="flex items-center gap-2">
                      <Badge variant={d.enabled ? "default":"warning"}>{d.enabled ? "Enabled":"Disabled"}</Badge>
                      <Button onClick={()=>rotateKey(d.id)} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1">🔄 Rotate Key</Button>
                      <Button onClick={()=>toggleDevice(d.id, !d.enabled)} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1">{d.enabled ? "Disable":"Enable"}</Button>
                    </div>
                  </div>
                  <div className="mt-2 p-2 rounded-md border bg-gray-50 font-mono text-xs break-all">API Key: {d.apiKey}</div>
                </div>
              ))}
            </div>
          }

          {/* Manual Usage → Charge a Channel */}
          <div className="rounded-xl border p-3">
            <div className="text-sm font-medium mb-2">Record Usage (charge a channel)</div>
            <div className="grid md:grid-cols-4 gap-2">
              <Select value={usage.meterId} onValueChange={(v)=>setUsage(u=>({...u, meterId:v}))}>
                <SelectTrigger><SelectValue placeholder="Meter" /></SelectTrigger>
                <SelectContent>{meters.map(m=><SelectItem key={m.id} value={m.id}>{m.name}</SelectItem>)}</SelectContent>
              </Select>
              <Input placeholder="Units (e.g. 100 calls)" value={usage.units} onChange={e=>setUsage(u=>({...u, units:e.target.value}))} />
              <Select value={usage.channelId} onValueChange={(v)=>setUsage(u=>({...u, channelId:v}))}>
                <SelectTrigger><SelectValue placeholder="Channel" /></SelectTrigger>
                <SelectContent>{channels.filter(c=>c.state==="Active").map(c=><SelectItem key={c.id} value={c.id}>{c.id} • {c.peer}</SelectItem>)}</SelectContent>
              </Select>
              <Button onClick={recordUsage} className="bg-green-600 hover:bg-green-700 text-xs px-2 py-1">⚡ Charge</Button>
            </div>
          </div>
        </div>
      </Card>

      {/* Invoices & Webhooks */}
      <div className="grid md:grid-cols-2 gap-4">
        <Card title="📦 Invoices / Pay Links">
          <div className="space-y-4">
            <div className="rounded-xl border p-3">
              <div className="grid md:grid-cols-4 gap-2">
                                 <Input placeholder="Recipient i..." value={invTo} onChange={e=>setInvTo(e.target.value)} />
                <Input placeholder="Amount (IPN)" value={invAmt} onChange={e=>setInvAmt(e.target.value)} />
                <Input placeholder="Memo (optional)" value={invMemo} onChange={e=>setInvMemo(e.target.value)} />
                <Button onClick={createInvoice} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1">➕ Create Link</Button>
              </div>
            </div>

            {invoices.length===0 ? <p className="text-sm text-gray-600">No invoices.</p> :
              <div className="space-y-2">
                {invoices.map(iv=>(
                  <div key={iv.id} className="rounded-xl border p-3 flex items-center justify-between">
                    <div>
                      <div className="font-medium">{iv.amount} IPN • {iv.memo || "No memo"}</div>
                      <div className="text-xs text-gray-600">To: {iv.to} • Status: {iv.status} • Expires: {iv.expiresAt}</div>
                      <div className="font-mono text-xs break-all">{iv.link}</div>
                    </div>
                    <Button onClick={()=>navigator.clipboard?.writeText(iv.link)} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1">Copy Link</Button>
                  </div>
                ))}
              </div>
            }
          </div>
        </Card>

        <Card title="🌐 Webhooks">
          <div className="space-y-3">
            {!wh ? <p className="text-sm">Loading…</p> : (
              <>
                <Label>Endpoint URL</Label>
                <Input value={wh.url} onChange={e=>setWh(w=>w ? {...w, url:e.target.value} : w)} placeholder="https://your-service/webhooks/ippan" />
                <Label>Events</Label>
                <div className="rounded-xl border p-2 max-h-32 overflow-y-auto">
                  {["channel.opened","channel.updated","channel.closed","invoice.paid","invoice.created","stream.started","stream.stopped"].map(ev=>(
                    <div key={ev} className="flex items-center gap-2 py-1">
                      <input
                        type="checkbox"
                        checked={wh.events.includes(ev)}
                        onChange={(e) => {
                          setWh(w=>{
                            if (!w) return w;
                            const has = w.events.includes(ev);
                            return { ...w, events: e.target.checked ? [...w.events, ev] : w.events.filter(e=>e!==ev) };
                          });
                        }}
                        className="w-4 h-4"
                      />
                      <span className="font-mono text-sm">{ev}</span>
                    </div>
                  ))}
                </div>
                <div className="text-xs text-gray-600">Signing secret: <span className="font-mono">{wh.secret}</span></div>
                <Button onClick={saveWebhook} className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1">🔄 Save Webhook</Button>
              </>
            )}
          </div>
        </Card>
      </div>

      {/* Activity */}
      <Card title="📈 Recent Transactions">
        <div className="divide-y">
          {activity.map(a=>(
            <div key={a.id} className="py-3 flex items-center justify-between">
              <div>
                <div className="font-medium">{a.title}</div>
                <div className="text-sm text-gray-600">{a.subtitle}</div>
              </div>
              <div className={`font-medium ${a.delta>0 ? "text-green-600" : a.delta<0 ? "text-red-600" : "text-gray-600"}`}>
                {a.delta>0?"+":""}{a.delta!==0?a.delta:"—"} IPN
              </div>
              <div className="text-sm text-gray-600 w-40 text-right">{a.when}</div>
              <div className="text-xs w-24 text-right">{a.status || ""}</div>
            </div>
          ))}
        </div>
      </Card>

             {/* Create Channel Sheet */}
       <Sheet open={openSheet} onOpenChange={setOpenSheet}>
         <SheetContent className="w-full sm:max-w-md">
           <SheetHeader>
             <SheetTitle>Create Channel</SheetTitle>
           </SheetHeader>
           <div className="py-6 space-y-6">
             <div className="space-y-2">
               <Label className="text-sm font-medium text-gray-700">Peer Address</Label>
               <Input 
                 value={openPeer} 
                 onChange={e=>setOpenPeer(e.target.value)} 
                 placeholder="i..." 
                 className="w-full"
               />
             </div>
             
             <div className="space-y-2">
               <Label className="text-sm font-medium text-gray-700">Initial Deposit (IPN)</Label>
               <Input 
                 value={openDeposit} 
                 onChange={e=>setOpenDeposit(e.target.value)} 
                 className="w-full"
               />
             </div>
             
             <div className="space-y-2">
               <Label className="text-sm font-medium text-gray-700">Challenge Period (hours)</Label>
               <Input 
                 value={openChallenge} 
                 onChange={e=>setOpenChallenge(e.target.value)} 
                 className="w-full"
               />
             </div>
           </div>
           <SheetFooter className="flex gap-3 pt-4">
             <Button onClick={()=>setOpenSheet(false)} className="bg-gray-600 hover:bg-gray-700">
               Cancel
             </Button>
             <Button onClick={openChannel} className="bg-green-600 hover:bg-green-700">
               Open Channel
             </Button>
           </SheetFooter>
         </SheetContent>
       </Sheet>
    </div>
  );
}
