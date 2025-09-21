import { useState, useEffect, useMemo, useRef } from 'react'
import { Card, Button, Input, Badge, Checkbox, Switch, Select, SelectTrigger, SelectValue, SelectContent, SelectItem, Sheet, SheetContent, SheetHeader, SheetTitle, SheetFooter, Label, Textarea } from '../components/UI'

// -----------------------------
// Types
// -----------------------------
type StoragePlan = {
  reservedGb: number;
  replication: 1 | 2 | 3 | 5;
  bandwidthCapGb: number;  // monthly cap
  autoRenew: boolean;
  coldStorage: boolean;
};

type StorageStats = {
  usedGb: number;
  reservedGb: number;
  files: number;
  replication: number;
  uptimePct: number;
  bandwidthUsedGb: number;
  pinnedCount: number;
};

type FileItem = {
  cid: string;
  name: string;
  sizeBytes: number;
  uploadedAt: string; // ISO
  replicated: number;
  folder?: string;
  public: boolean;
  status: "Stored" | "Pinning" | "Archived";
  versions?: number;
};

type UploadedFile = {
  cid: string;
  name: string;
  sizeBytes: number;
  uploadedAt: string; // ISO
  uploadedBy: string; // Node ID
  nodeLabel?: string; // Human-readable node label
  status: "Stored" | "Pinning" | "Archived";
  public: boolean;
};

type UploadOpts = {
  encrypt: boolean;
  passphrase?: string;
  folder?: string;
  pin: boolean;
  ttlDays?: number; // lifecycle retention
};

// NEW types for Neural Datasets
type NeuralDataset = {
  id: string;
  name: string;
  modality: "image" | "text" | "audio" | "tabular";
  labels: string[];
  visibility: "private" | "public" | "commercial";
  items: number;
};

type DatasetImportOpts = {
  datasetId: string;
  cids: string[];
  mode: "link" | "copy";                 // link=zero-copy from Storage; copy=physically duplicate
  inferLabelFromFolder: boolean;
  applyLabel?: string;
  labels?: string[];
  splits: { train: number; val: number; test: number };
  dedupe: boolean;
  normalizeImages: boolean;
  tokenizeText: boolean;
  description?: string;
  license?: "private" | "public" | "commercial";
};

// -----------------------------
// Mock APIs (replace with real endpoints)
// -----------------------------
function fmtSize(n: number) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 ** 2) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 ** 3) return `${(n / 1024 ** 2).toFixed(1)} MB`;
  return `${(n / 1024 ** 3).toFixed(2)} GB`;
}

function ipnEstimate(plan: StoragePlan): number {
  const basePerGb = plan.coldStorage ? 0.02 : 0.05; // IPN / GB / month (mock)
  const replMult = plan.replication;                // linear factor
  return Number((plan.reservedGb * basePerGb * replMult).toFixed(2));
}

async function apiGetPlan(): Promise<StoragePlan> {
  return new Promise((r) =>
    setTimeout(() => r({ reservedGb: 50, replication: 3, bandwidthCapGb: 200, autoRenew: true, coldStorage: false }), 160)
  );
}

async function apiUpdatePlan(plan: StoragePlan): Promise<{ ok: boolean }> {
  console.log("UPDATE PLAN", plan);
  return new Promise((r) => setTimeout(() => r({ ok: true }), 200));
}

async function apiGetStats(): Promise<StorageStats> {
  return new Promise((r) =>
    setTimeout(
      () =>
        r({
          usedGb: 22.5,
          reservedGb: 50,
          files: 156,
          replication: 3,
          uptimePct: 99.9,
          bandwidthUsedGb: 48,
          pinnedCount: 87,
        }),
      180
    )
  );
}

async function apiGetFolders(): Promise<string[]> {
  return new Promise((r) => setTimeout(() => r(["root", "invoices", "photos", "datasets"]), 120));
}

async function apiCreateFolder(name: string): Promise<{ ok: boolean }> {
  console.log("CREATE FOLDER", name);
  return new Promise((r) => setTimeout(() => r({ ok: true }), 160));
}

async function apiListFiles(query = "", folder?: string): Promise<FileItem[]> {
  const rows: FileItem[] = [
    { cid: "bafy1", name: "document.pdf", sizeBytes: 2.3 * 1024 ** 2, uploadedAt: new Date(Date.now() - 2 * 3600e3).toISOString(), replicated: 3, folder: "root", public: false, status: "Stored", versions: 3 },
    { cid: "bafy2", name: "image.jpg", sizeBytes: 1.8 * 1024 ** 2, uploadedAt: new Date(Date.now() - 24 * 3600e3).toISOString(), replicated: 3, folder: "photos", public: true, status: "Stored", versions: 1 },
    { cid: "bafy3", name: "dataset.csv", sizeBytes: 120 * 1024 ** 2, uploadedAt: new Date(Date.now() - 3 * 24 * 3600e3).toISOString(), replicated: 5, folder: "datasets", public: false, status: "Pinning", versions: 5 },
  ];
  let f = rows;
  if (folder && folder !== "all") f = f.filter((x) => (x.folder || "root") === folder);
  if (query) f = f.filter((x) => x.name.toLowerCase().includes(query.toLowerCase()) || x.cid.includes(query));
  return new Promise((r) => setTimeout(() => r(f), 160));
}

async function apiListUploadedFiles(): Promise<UploadedFile[]> {
  const rows: UploadedFile[] = [
    { cid: "bafy4", name: "research_paper.pdf", sizeBytes: 5.2 * 1024 ** 2, uploadedAt: new Date(Date.now() - 1 * 3600e3).toISOString(), uploadedBy: "node-abc123", nodeLabel: "Research Node Alpha", status: "Stored", public: false },
    { cid: "bafy5", name: "model_weights.bin", sizeBytes: 45.8 * 1024 ** 2, uploadedAt: new Date(Date.now() - 6 * 3600e3).toISOString(), uploadedBy: "node-def456", nodeLabel: "AI Training Node", status: "Stored", public: true },
    { cid: "bafy6", name: "dataset_v2.json", sizeBytes: 12.1 * 1024 ** 2, uploadedAt: new Date(Date.now() - 12 * 3600e3).toISOString(), uploadedBy: "node-ghi789", nodeLabel: "Data Provider Beta", status: "Pinning", public: false },
    { cid: "bafy7", name: "backup_archive.tar.gz", sizeBytes: 89.3 * 1024 ** 2, uploadedAt: new Date(Date.now() - 2 * 24 * 3600e3).toISOString(), uploadedBy: "node-jkl012", nodeLabel: "Backup Node Gamma", status: "Stored", public: false },
    { cid: "bafy8", name: "media_collection.zip", sizeBytes: 156.7 * 1024 ** 2, uploadedAt: new Date(Date.now() - 3 * 24 * 3600e3).toISOString(), uploadedBy: "node-mno345", nodeLabel: "Media Node Delta", status: "Archived", public: true },
  ];
  return new Promise((r) => setTimeout(() => r(rows), 160));
}

async function apiUploadFile(file: File, opts: UploadOpts): Promise<FileItem> {
  console.log("UPLOAD", file.name, opts);
  const cid = "bafy" + Math.random().toString(16).slice(2, 8);
  return new Promise((r) =>
    setTimeout(
      () =>
        r({
          cid,
          name: file.name,
          sizeBytes: file.size,
          uploadedAt: new Date().toISOString(),
          replicated: opts.pin ? 3 : 1,
          folder: opts.folder || "root",
          public: false,
          status: "Pinning",
          versions: 1,
        }),
      600
    )
  );
}

async function apiTogglePublic(cid: string, makePublic: boolean): Promise<{ ok: boolean; url?: string }> {
  console.log("PUBLIC", cid, makePublic);
  const url = makePublic ? `https://gateway.ippan/ipfs/${cid}` : undefined;
  return new Promise((r) => setTimeout(() => r({ ok: true, url }), 140));
}

async function apiDeleteFile(cid: string): Promise<{ ok: boolean }> {
  console.log("DELETE", cid);
  return new Promise((r) => setTimeout(() => r({ ok: true }), 140));
}

async function apiRepair(cid: string): Promise<{ ok: boolean }> {
  console.log("REPAIR", cid);
  return new Promise((r) => setTimeout(() => r({ ok: true }), 180));
}

async function apiCreateShareLink(cid: string, expiresDays: number): Promise<{ ok: boolean; url: string }> {
  const url = `https://share.ippan/${cid}?exp=${expiresDays}`;
  return new Promise((r) => setTimeout(() => r({ ok: true, url }), 140));
}

// Provider mode (optional: contribute disk to network)
async function apiGetProvider(): Promise<{ enabled: boolean; contributeGb: number; nodeId?: string; status?: "Online" | "Offline" }> {
  return new Promise((r) => setTimeout(() => r({ enabled: false, contributeGb: 0 }), 120));
}

async function apiSetProvider(enabled: boolean, contributeGb: number): Promise<{ ok: boolean; nodeId: string; status: "Online" | "Offline" }> {
  const nodeId = "ipnworker-" + Math.random().toString(16).slice(2, 8);
  return new Promise((r) => setTimeout(() => r({ ok: true, nodeId, status: enabled ? "Online" : "Offline" }), 240));
}

// === Neural Dataset APIs (mock) ===
async function apiListNeuralDatasets(): Promise<NeuralDataset[]> {
  return new Promise((r) =>
    setTimeout(
      () =>
        r([
          { id: "ds1", name: "Alpha Images", modality: "image", labels: ["cat","dog"], visibility: "private", items: 1200 },
          { id: "ds2", name: "Docs TXT", modality: "text", labels: ["policy","invoice"], visibility: "public", items: 540 },
        ]),
      120
    )
  );
}

async function apiCreateNeuralDataset(name: string, modality: NeuralDataset["modality"], visibility: NeuralDataset["visibility"], description?: string, labels?: string[]
): Promise<{ ok: boolean; dataset?: NeuralDataset }> {
  const ds: NeuralDataset = { id: "ds-" + Math.random().toString(16).slice(2,8), name, modality, labels: labels || [], visibility, items: 0 };
  return new Promise((r) => setTimeout(() => r({ ok: true, dataset: ds }), 200));
}

async function apiImportToNeural(payload: DatasetImportOpts): Promise<{ ok: boolean; imported: number }> {
  console.log("IMPORT→NEURAL", payload);
  // pretend all imported
  return new Promise((r) => setTimeout(() => r({ ok: true, imported: payload.cids.length }), 600));
}

// -----------------------------
// Helpers
// -----------------------------
function pct(n: number, d: number) {
  if (d <= 0) return 0;
  return Math.min(100, Math.round((n / d) * 100));
}

function ProgressBar({ value }: { value: number }) {
  return (
    <div className="w-full h-2 rounded-full bg-gray-200 overflow-hidden">
      <div className="h-2 bg-blue-600" style={{ width: `${Math.max(0, Math.min(100, value))}%` }} />
    </div>
  );
}

// -----------------------------
// Component
// -----------------------------
export default function StoragePage() {
  // Plan & stats
  const [plan, setPlan] = useState<StoragePlan | null>(null);
  const [stats, setStats] = useState<StorageStats | null>(null);
  const [savingPlan, setSavingPlan] = useState(false);

  // Folders
  const [folders, setFolders] = useState<string[]>(["root"]);
  const [newFolder, setNewFolder] = useState("");
  const [folderSel, setFolderSel] = useState<string>("all");

  // Files
  const [files, setFiles] = useState<FileItem[]>([]);
  const [query, setQuery] = useState("");
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  // Monitoring uploaded files from other nodes
  const [uploadedFiles, setUploadedFiles] = useState<UploadedFile[]>([]);
  const [monitoringEnabled, setMonitoringEnabled] = useState(true);
  const [encrypt, setEncrypt] = useState(true);
  const [pass, setPass] = useState("");
  const [pin, setPin] = useState(true);
  const [ttlDays, setTtlDays] = useState<number>(0); // 0 = keep forever
  const [uploading, setUploading] = useState(false);

  // Share
  const [shareCid, setShareCid] = useState<string | null>(null);
  const [shareDays, setShareDays] = useState(7);
  const [shareUrl, setShareUrl] = useState<string>("");
  const [shareModalOpen, setShareModalOpen] = useState(false);

  // Provider
  const [providerEnabled, setProviderEnabled] = useState(false);
  const [providerGb, setProviderGb] = useState(0);
  const [providerNode, setProviderNode] = useState<{ nodeId?: string; status?: "Online" | "Offline" }>({});

  // === Selection for Files → Datasets ===
  const [selectedCids, setSelectedCids] = useState<Set<string>>(new Set());

  // Dataset sheet (side drawer)
  const [dsOpen, setDsOpen] = useState(false);
  const [datasets, setDatasets] = useState<NeuralDataset[]>([]);
  const [useNewDataset, setUseNewDataset] = useState(false);
  const [dsId, setDsId] = useState<string>("");
  const [dsName, setDsName] = useState<string>("");
  const [dsDesc, setDsDesc] = useState<string>("");
  const [dsModality, setDsModality] = useState<NeuralDataset["modality"]>("image");
  const [dsVisibility, setDsVisibility] = useState<NeuralDataset["visibility"]>("private");
  const [dsLabels, setDsLabels] = useState<string>(""); // comma-separated
  const [mode, setMode] = useState<"link"|"copy">("link");
  const [inferLabelFromFolder, setInferLabelFromFolder] = useState(true);
  const [applyLabel, setApplyLabel] = useState<string>("");
  const [splitTrain, setSplitTrain] = useState<number>(80);
  const [splitVal, setSplitVal] = useState<number>(10);
  const [splitTest, setSplitTest] = useState<number>(10);
  const [dedupe, setDedupe] = useState(true);
  const [normalizeImages, setNormalizeImages] = useState(true);
  const [tokenizeText, setTokenizeText] = useState(true);
  const [creatingDs, setCreatingDs] = useState(false);
  const [importingDs, setImportingDs] = useState(false);

  // Load initial
  useEffect(() => {
    let alive = true;
    (async () => {
      const [p, s, f, pr, ds, uf] = await Promise.all([apiGetPlan(), apiGetStats(), apiGetFolders(), apiGetProvider(), apiListNeuralDatasets(), apiListUploadedFiles()]);
      if (!alive) return;
      setPlan(p);
      setStats(s);
      setFolders(["all", ...new Set(["root", ...f])]);
      setFolderSel("all");
      setProviderEnabled(pr.enabled);
      setProviderGb(pr.contributeGb);
      setProviderNode({ nodeId: pr.nodeId, status: pr.status });
      setDatasets(ds);
      setUploadedFiles(uf);
      const ls = await apiListFiles("", "all");
      setFiles(ls);
    })();
    return () => {
      alive = false;
    };
  }, []);

  // Helper functions for dataset import
  function toggleCid(id: string, checked: boolean) {
    setSelectedCids(prev => {
      const next = new Set(prev);
      if (checked) next.add(id); else next.delete(id);
      return next;
    });
  }
  
  function clearSelection() { 
    setSelectedCids(new Set()); 
  }
  
  const splitsTotal = splitTrain + splitVal + splitTest;

  // Re-query files
  useEffect(() => {
    let alive = true;
    apiListFiles(query, folderSel === "all" ? undefined : folderSel).then((rows) => {
      if (!alive) return;
      setFiles(rows);
    });
    return () => {
      alive = false;
    };
  }, [query, folderSel]);

  const priceEst = useMemo(() => (plan ? ipnEstimate(plan) : 0), [plan]);

  async function savePlan() {
    if (!plan) return;
    setSavingPlan(true);
    const ok = (await apiUpdatePlan(plan)).ok;
    setSavingPlan(false);
    if (ok) {
      const s = await apiGetStats();
      setStats(s);
      alert("✅ Storage plan updated.");
    }
  }

  async function createFolder() {
    const name = newFolder.trim();
    if (!name) return;
    const ok = (await apiCreateFolder(name)).ok;
    if (ok) {
      setFolders((prev) => Array.from(new Set([...prev, name])));
      setNewFolder("");
      setFolderSel(name);
    }
  }

  async function onUpload(e: React.ChangeEvent<HTMLInputElement>) {
    const filesList = e.target.files;
    if (!filesList || filesList.length === 0) return;
    
    setUploading(true);
    const added: FileItem[] = [];
    const errors: string[] = [];
    
    try {
      for (const f of Array.from(filesList)) {
        try {
          console.log(`Starting upload for file: ${f.name} (${f.size} bytes)`);
          
          // NOTE: chunking/resumable would be handled in a real uploader; mocked here
          const it = await apiUploadFile(f, {
            encrypt,
            passphrase: encrypt && pass ? pass : undefined,
            folder: folderSel === "all" ? "root" : folderSel,
            pin,
            ttlDays: ttlDays || undefined,
          });
          
          console.log(`Upload completed for file: ${f.name}, CID: ${it.cid}`);
          added.push(it);
        } catch (error) {
          console.error(`Upload failed for file: ${f.name}`, error);
          errors.push(`${f.name}: ${error instanceof Error ? error.message : 'Unknown error'}`);
        }
      }
      
      // Add successfully uploaded files to the list
      if (added.length > 0) {
        setFiles((prev) => [ ...added, ...prev ]);
        // refresh stats
        const s = await apiGetStats();
        setStats(s);
      }
      
      // Show results
      if (errors.length > 0) {
        alert(`Upload completed with errors:\n${errors.join('\n')}`);
      } else if (added.length > 0) {
        alert(`✅ Successfully uploaded ${added.length} file(s)`);
      }
      
    } catch (error) {
      console.error('Upload process failed:', error);
      alert(`Upload failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    } finally {
      setUploading(false);
      if (fileInputRef.current) fileInputRef.current.value = "";
    }
  }

  async function togglePublic(f: FileItem) {
    const res = await apiTogglePublic(f.cid, !f.public);
    setFiles((prev) => prev.map((x) => (x.cid === f.cid ? { ...x, public: !x.public } : x)));
    if (res.url && !f.public) {
      navigator.clipboard?.writeText(res.url);
      alert("🔗 Public link copied:\n" + res.url);
    }
  }

  async function deleteFile(cid: string) {
    if (!confirm("Delete this file?")) return;
    const ok = (await apiDeleteFile(cid)).ok;
    if (ok) setFiles((prev) => prev.filter((x) => x.cid !== cid));
    const s = await apiGetStats();
    setStats(s);
  }

  async function repairFile(cid: string) {
    const ok = (await apiRepair(cid)).ok;
    if (ok) alert("🛠️ Repair requested.");
  }

  async function makeShareLink() {
    if (!shareCid) return;
    const res = await apiCreateShareLink(shareCid, shareDays);
    if (res.ok) {
      setShareUrl(res.url);
      navigator.clipboard?.writeText(res.url);
    }
  }

  async function setProvider() {
    const res = await apiSetProvider(providerEnabled, providerGb);
    if (res.ok) {
      setProviderNode({ nodeId: res.nodeId, status: res.status });
      alert(`Provider ${providerEnabled ? "enabled" : "disabled"} (${res.status}).`);
    }
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">📁 Storage</h1>
          <p className="text-sm text-gray-600">
            Connected to IPPAN Storage Network
          </p>
        </div>
      </div>

      {/* Plan & Commitment */}
      <Card title="⚙️ Reserved Space & Plan">
        <div className="space-y-4">
          {!plan ? (
            <p className="text-sm">Loading plan…</p>
          ) : (
            <>
              <div className="grid md:grid-cols-2 gap-4">
                <div className="space-y-3">
                  <div>
                    <label className="text-sm font-medium">Reserved Space (GB)</label>
                    <div className="flex items-center gap-3 mt-1">
                      <input 
                        type="range" 
                        min="5" 
                        max="1024" 
                        step="5" 
                        value={plan.reservedGb} 
                        onChange={(e) => setPlan({ ...plan, reservedGb: Number(e.target.value) })} 
                        className="w-64" 
                      />
                      <Input 
                        className="w-24" 
                        type="number" 
                        value={plan.reservedGb} 
                        onChange={(e) => setPlan({ ...plan, reservedGb: Math.max(1, Number(e.target.value) || plan.reservedGb) })} 
                      />
                    </div>
                  </div>

                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <label className="text-sm font-medium">Replication Factor</label>
                      <select 
                        value={plan.replication} 
                        onChange={(e) => setPlan({ ...plan, replication: Number(e.target.value) as any })}
                        className="w-full border rounded px-3 py-2 mt-1"
                      >
                        <option value="1">1x (No redundancy)</option>
                        <option value="2">2x</option>
                        <option value="3">3x (Standard)</option>
                        <option value="5">5x (High SLA)</option>
                      </select>
                    </div>
                    <div>
                      <label className="text-sm font-medium">Bandwidth Cap (GB / month)</label>
                      <Input 
                        type="number" 
                        value={plan.bandwidthCapGb} 
                        onChange={(e) => setPlan({ ...plan, bandwidthCapGb: Math.max(0, Number(e.target.value) || 0) })} 
                      />
                    </div>
                  </div>

                  <div className="flex items-center gap-6">
                    <div className="flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={plan.autoRenew}
                        onChange={(e) => setPlan({ ...plan, autoRenew: e.target.checked })}
                        className="w-4 h-4"
                      />
                      <span className="text-sm">Auto-renew</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={plan.coldStorage}
                        onChange={(e) => setPlan({ ...plan, coldStorage: e.target.checked })}
                        className="w-4 h-4"
                      />
                      <span className="text-sm">Cold storage (lower cost, slower retrieval)</span>
                    </div>
                  </div>
                </div>

                <div className="space-y-3">
                  <div className="rounded-xl border p-3">
                    <div className="text-sm text-gray-600">Monthly Estimate</div>
                    <div className="text-2xl font-semibold">{priceEst} <span className="text-base font-normal">IPN / mo</span></div>
                    <div className="text-xs text-gray-600">Includes replication factor ×{plan.replication}</div>
                  </div>

                  <div className="rounded-xl border p-3">
                    <div className="text-sm text-gray-600">Lifecycle</div>
                    <div className="text-xs mb-1">Default retention for new uploads</div>
                    <div className="flex items-center gap-2">
                      <label className="text-sm">Delete after (days)</label>
                      <Input 
                        className="w-28" 
                        type="number" 
                        value={ttlDays} 
                        onChange={(e) => setTtlDays(Math.max(0, Number(e.target.value) || 0))} 
                      />
                      <span className="text-xs text-gray-600">{ttlDays === 0 ? "Keep forever" : "Auto-delete after set period"}</span>
                    </div>
                  </div>

                  <Button onClick={savePlan} disabled={savingPlan}>
                    {savingPlan ? "Saving…" : "Commit / Update Plan"}
                  </Button>
                </div>
              </div>
            </>
          )}
        </div>
      </Card>

      {/* Upload */}
      <Card title="📤 Upload File">
        <div className="space-y-4">
          <div className="grid md:grid-cols-3 gap-4">
            <div className="space-y-2 md:col-span-2">
              <label className="text-sm font-medium">Select File</label>
              <input 
                type="file" 
                ref={fileInputRef} 
                multiple 
                onChange={onUpload}
                className="hidden"
                accept="*/*"
              />
              <div className="border-2 border-dashed border-gray-300 rounded-lg p-6 text-center">
                <div className="text-gray-600 mb-2">
                  {uploading ? "Uploading files..." : "Click 'Upload File' button to select files"}
                </div>
                <div className="text-xs text-gray-500">
                  Client-side chunking & resumable upload (mock). Encryption optional.
                </div>
              </div>
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Folder</label>
              <select 
                value={folderSel} 
                onChange={(e) => setFolderSel(e.target.value)}
                className="w-full border rounded px-3 py-2"
              >
                {folders.map((f) => <option key={f} value={f}>{f}</option>)}
              </select>
              <div className="flex gap-2 mt-2">
                <Input 
                  placeholder="New folder…" 
                  value={newFolder} 
                  onChange={(e) => setNewFolder(e.target.value)} 
                />
                <Button onClick={createFolder} className="bg-gray-600 hover:bg-gray-700">
                  📁 Create
                </Button>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-3 gap-4">
            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={encrypt}
                onChange={(e) => setEncrypt(e.target.checked)}
                className="w-4 h-4"
              />
              <span className="text-sm">🔒 Encrypt before upload</span>
            </div>
            <Input 
              disabled={!encrypt} 
              value={pass} 
              onChange={(e) => setPass(e.target.value)} 
              placeholder="Passphrase (optional)" 
            />
            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={pin}
                onChange={(e) => setPin(e.target.checked)}
                className="w-4 h-4"
              />
              <span className="text-sm">🔑 Pin for availability</span>
            </div>
          </div>

          <Button 
            disabled={uploading} 
            onClick={() => {
              if (fileInputRef.current) {
                fileInputRef.current.click();
              }
            }}
            className="w-full"
          >
            {uploading ? "⏳ Uploading…" : "📁 Select & Upload Files"}
          </Button>
        </div>
      </Card>

      {/* Stats */}
      <Card title="📊 Storage Statistics">
        <div className="space-y-4">
          {!stats ? (
            <p className="text-sm">Loading stats…</p>
          ) : (
            <div className="grid md:grid-cols-4 gap-4">
              <div className="rounded-xl border p-3">
                <div className="text-sm text-gray-600">Used / Reserved</div>
                <div className="text-xl font-semibold">{stats.usedGb} / {stats.reservedGb} GB</div>
                <ProgressBar value={pct(stats.usedGb, stats.reservedGb)} />
              </div>
              <div className="rounded-xl border p-3">
                <div className="text-sm text-gray-600">Files Stored</div>
                <div className="text-xl font-semibold">{stats.files.toLocaleString()}</div>
              </div>
              <div className="rounded-xl border p-3">
                <div className="text-sm text-gray-600">Replication</div>
                <div className="text-xl font-semibold">×{stats.replication}</div>
              </div>
              <div className="rounded-xl border p-3">
                <div className="text-sm text-gray-600">Uptime</div>
                <div className="text-xl font-semibold">{stats.uptimePct}%</div>
              </div>
              <div className="rounded-xl border p-3">
                <div className="text-sm text-gray-600">Bandwidth (mo)</div>
                <div className="text-xl font-semibold">{stats.bandwidthUsedGb} GB</div>
                <div className="text-xs text-gray-600">Cap: {plan?.bandwidthCapGb ?? 0} GB</div>
              </div>
              <div className="rounded-xl border p-3">
                <div className="text-sm text-gray-600">Pinned</div>
                <div className="text-xl font-semibold">{stats.pinnedCount}</div>
              </div>
            </div>
          )}
        </div>
      </Card>

      {/* Files */}
      <Card title="🗄️ Files">
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-gray-600">🔍</span>
              <Input 
                placeholder="Search files or CIDs…" 
                value={query} 
                onChange={(e) => setQuery(e.target.value)} 
                className="max-w-md"
              />
            </div>
            <Button
              disabled={selectedCids.size === 0}
              onClick={() => setDsOpen(true)}
              className="bg-green-600 hover:bg-green-700"
            >
              🧠 Import to Dataset ({selectedCids.size})
            </Button>
          </div>
          
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead className="text-left text-gray-600">
                <tr>
                  <th className="py-2 pr-3">
                    <Checkbox
                      checked={files.length > 0 && selectedCids.size === files.length}
                      onChange={(e) => {
                        if (e.target.checked) setSelectedCids(new Set(files.map(f => f.cid)));
                        else clearSelection();
                      }}
                    />
                  </th>
                  <th className="py-2 pr-3">Name</th>
                  <th className="py-2 pr-3">CID</th>
                  <th className="py-2 pr-3">Size</th>
                  <th className="py-2 pr-3">Folder</th>
                  <th className="py-2 pr-3">Replicated</th>
                  <th className="py-2 pr-3">Uploaded</th>
                  <th className="py-2 pr-3">Status</th>
                  <th className="py-2 pr-3">Privacy</th>
                  <th className="py-2 pr-3">Actions</th>
                </tr>
              </thead>
              <tbody>
                {files.length === 0 ? (
                  <tr><td className="py-3" colSpan={10}>No files.</td></tr>
                ) : files.map((f) => (
                  <tr key={f.cid} className="border-t">
                    <td className="py-2 pr-3">
                      <Checkbox
                        checked={selectedCids.has(f.cid)}
                        onChange={(e) => toggleCid(f.cid, e.target.checked)}
                      />
                    </td>
                    <td className="py-2 pr-3">
                      <div className="font-medium">{f.name}</div>
                      {!!f.versions && <div className="text-xs text-gray-600">versions: {f.versions}</div>}
                    </td>
                    <td className="py-2 pr-3 font-mono text-xs break-all">{f.cid}</td>
                    <td className="py-2 pr-3">{fmtSize(f.sizeBytes)}</td>
                    <td className="py-2 pr-3">{f.folder || "root"}</td>
                    <td className="py-2 pr-3">×{f.replicated}</td>
                    <td className="py-2 pr-3">{new Date(f.uploadedAt).toLocaleString()}</td>
                    <td className="py-2 pr-3">
                      <Badge variant={f.status === "Stored" ? "default" : f.status === "Pinning" ? "warning" : "error"}>
                        {f.status}
                      </Badge>
                    </td>
                    <td className="py-2 pr-3">
                      <Badge variant={f.public ? "default" : "warning"}>
                        {f.public ? "Public" : "Private"}
                      </Badge>
                    </td>
                    <td className="py-2 pr-3">
                      <div className="flex flex-wrap gap-2">
                                                 <Button 
                           onClick={() => togglePublic(f)}
                           className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1"
                         >
                           {f.public ? "Make Private" : "Make Public"}
                         </Button>
                         <Button 
                           onClick={() => { setShareCid(f.cid); setShareUrl(""); setShareModalOpen(true); }}
                           className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1"
                         >
                           🔗 Share
                         </Button>
                         <Button 
                           onClick={() => navigator.clipboard?.writeText(f.cid)}
                           className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1"
                         >
                           📋 Copy CID
                         </Button>
                         <Button 
                           onClick={() => repairFile(f.cid)}
                           className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1"
                         >
                           🔧 Repair
                         </Button>
                         <Button 
                           onClick={() => deleteFile(f.cid)}
                           className="bg-red-600 hover:bg-red-700 text-xs px-2 py-1"
                         >
                           🗑️ Delete
                         </Button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </Card>

      {/* Provider Mode (optional) */}
      <Card title="☁️ Provide Storage (Earn)">
        <div className="space-y-4">
          <div className="grid md:grid-cols-3 gap-4">
            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={providerEnabled}
                onChange={(e) => setProviderEnabled(e.target.checked)}
                className="w-4 h-4"
              />
              <span className="text-sm">Enable my node as storage provider</span>
            </div>
            <div className="space-y-1">
              <label className="text-sm font-medium">Contributed Capacity (GB)</label>
              <div className="flex items-center gap-3">
                <input 
                  type="range"
                  min="10"
                  max="4096"
                  step="10"
                  value={providerGb}
                  onChange={(e) => setProviderGb(Number(e.target.value))}
                  className="w-56"
                />
                <Input 
                  className="w-24" 
                  type="number" 
                  value={providerGb} 
                  onChange={(e) => setProviderGb(Math.max(0, Number(e.target.value) || 0))} 
                />
              </div>
              <div className="text-xs text-gray-600">Estimated rewards: ~{(providerGb * 0.01).toFixed(2)} IPN / mo (mock)</div>
            </div>
            <div className="flex items-center gap-3">
              <Button onClick={setProvider}>
                🔄 Apply
              </Button>
              {providerNode.nodeId && (
                <div className="text-xs">
                  Node: <span className="font-mono">{providerNode.nodeId}</span><br />
                  Status: <span className="font-medium">{providerNode.status}</span>
                </div>
              )}
            </div>
          </div>
          <p className="text-xs text-gray-600">
            Your device must stay online to maintain replication SLAs. Rewards depend on uptime, bandwidth, and audited proofs.
          </p>
        </div>
      </Card>

      {/* Node Upload Monitoring */}
      <Card title="📡 Files Uploaded by Other Nodes">
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Switch
                checked={monitoringEnabled}
                onCheckedChange={setMonitoringEnabled}
              />
              <span className="text-sm font-medium">Enable Monitoring</span>
            </div>
            <div className="text-sm text-gray-600">
              {uploadedFiles.length} files from {new Set(uploadedFiles.map(f => f.uploadedBy)).size} nodes
            </div>
          </div>

          {monitoringEnabled ? (
            <div className="space-y-3">
              {uploadedFiles.length === 0 ? (
                <div className="text-center text-gray-500 py-8">
                  <div className="text-4xl mb-2">📭</div>
                  <div>No files uploaded by other nodes yet</div>
                  <div className="text-sm">Files uploaded to your node will appear here</div>
                </div>
              ) : (
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead className="text-left text-gray-600">
                      <tr>
                        <th className="py-2 pr-3">Name</th>
                        <th className="py-2 pr-3">CID</th>
                        <th className="py-2 pr-3">Size</th>
                        <th className="py-2 pr-3">Uploaded By</th>
                        <th className="py-2 pr-3">Uploaded At</th>
                        <th className="py-2 pr-3">Status</th>
                        <th className="py-2 pr-3">Privacy</th>
                        <th className="py-2 pr-3">Actions</th>
                      </tr>
                    </thead>
                    <tbody>
                      {uploadedFiles.map((file) => (
                        <tr key={file.cid} className="border-t hover:bg-gray-50">
                          <td className="py-2 pr-3">
                            <div className="font-medium">{file.name}</div>
                          </td>
                          <td className="py-2 pr-3 font-mono text-xs break-all">{file.cid}</td>
                          <td className="py-2 pr-3">{fmtSize(file.sizeBytes)}</td>
                          <td className="py-2 pr-3">
                            <div className="text-sm">
                              <div className="font-medium">{file.nodeLabel || file.uploadedBy}</div>
                              <div className="text-xs text-gray-500 font-mono">{file.uploadedBy}</div>
                            </div>
                          </td>
                          <td className="py-2 pr-3">{new Date(file.uploadedAt).toLocaleString()}</td>
                          <td className="py-2 pr-3">
                            <Badge variant={file.status === "Stored" ? "default" : file.status === "Pinning" ? "warning" : "error"}>
                              {file.status}
                            </Badge>
                          </td>
                          <td className="py-2 pr-3">
                            <Badge variant={file.public ? "default" : "warning"}>
                              {file.public ? "Public" : "Private"}
                            </Badge>
                          </td>
                          <td className="py-2 pr-3">
                            <div className="flex flex-wrap gap-2">
                              <Button 
                                onClick={() => navigator.clipboard?.writeText(file.cid)}
                                className="bg-gray-600 hover:bg-gray-700 text-xs px-2 py-1"
                              >
                                📋 Copy CID
                              </Button>
                              {file.public && (
                                <Button 
                                  onClick={() => window.open(`https://ipfs.io/ipfs/${file.cid}`, '_blank')}
                                  className="bg-blue-600 hover:bg-blue-700 text-xs px-2 py-1"
                                >
                                  🌐 View
                                </Button>
                              )}
                            </div>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          ) : (
            <div className="text-center text-gray-500 py-8">
              <div className="text-4xl mb-2">🔇</div>
              <div>Monitoring is disabled</div>
              <div className="text-sm">Enable monitoring to see files uploaded by other nodes</div>
            </div>
          )}
        </div>
      </Card>

      {/* Dataset Import Sheet */}
      <Sheet open={dsOpen} onOpenChange={setDsOpen}>
        <SheetContent className="w-full sm:max-w-xl">
          <SheetHeader>
            <SheetTitle className="flex items-center gap-2">
              ☁️ Import to Neural → Datasets
            </SheetTitle>
          </SheetHeader>

          <div className="py-4 space-y-4">
            {/* Pick existing OR create new */}
            <div className="rounded-xl border p-3 space-y-3">
              <div className="flex items-center justify-between">
                <div className="text-sm font-medium">Target Dataset</div>
                <div className="flex items-center gap-2">
                  <Switch checked={useNewDataset} onCheckedChange={setUseNewDataset} />
                  <span className="text-sm">Create new</span>
                </div>
              </div>

              {!useNewDataset ? (
                <div className="space-y-2">
                  <Label>Select existing</Label>
                  <Select value={dsId} onValueChange={setDsId}>
                    <SelectTrigger><SelectValue placeholder="Choose dataset" /></SelectTrigger>
                    <SelectContent>
                      {datasets.map(d => (
                        <SelectItem key={d.id} value={d.id}>
                          {d.name} • {d.modality} • {d.items} items
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              ) : (
                <div className="space-y-2">
                  <Label>Name</Label>
                  <Input value={dsName} onChange={(e)=>setDsName(e.target.value)} placeholder="My Dataset" />
                  <div className="grid grid-cols-2 gap-2">
                    <div>
                      <Label>Modality</Label>
                      <Select value={dsModality} onValueChange={(v)=>setDsModality(v as any)}>
                        <SelectTrigger><SelectValue /></SelectTrigger>
                        <SelectContent>
                          <SelectItem value="image">image</SelectItem>
                          <SelectItem value="text">text</SelectItem>
                          <SelectItem value="audio">audio</SelectItem>
                          <SelectItem value="tabular">tabular</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                    <div>
                      <Label>Visibility</Label>
                      <Select value={dsVisibility} onValueChange={(v)=>setDsVisibility(v as any)}>
                        <SelectTrigger><SelectValue /></SelectTrigger>
                        <SelectContent>
                          <SelectItem value="private">private</SelectItem>
                          <SelectItem value="public">public</SelectItem>
                          <SelectItem value="commercial">commercial</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  </div>
                  <Label>Description</Label>
                  <Textarea value={dsDesc} onChange={(e)=>setDsDesc(e.target.value)} placeholder="Short description…" />
                  <Label className="flex items-center gap-1">🏷️ Labels (comma-separated)</Label>
                  <Input value={dsLabels} onChange={(e)=>setDsLabels(e.target.value)} placeholder="cat,dog,car,..." />
                  <Button
                    disabled={!dsName}
                    onClick={async () => {
                      setCreatingDs(true);
                      const labels = dsLabels.split(",").map(s=>s.trim()).filter(Boolean);
                      const res = await apiCreateNeuralDataset(dsName, dsModality, dsVisibility, dsDesc, labels);
                      setCreatingDs(false);
                      if (res.ok && res.dataset) {
                        setDatasets(prev => [res.dataset!, ...prev]);
                        setUseNewDataset(false);
                        setDsId(res.dataset!.id);
                      } else alert("Failed to create dataset");
                    }}
                  >
                    📝 Create & Select
                  </Button>
                </div>
              )}
            </div>

            {/* Labeling options */}
            <div className="rounded-xl border p-3 space-y-3">
              <div className="text-sm font-medium flex items-center gap-2">🏷️ Labeling</div>
              <div className="flex items-center gap-2">
                <Switch checked={inferLabelFromFolder} onCheckedChange={setInferLabelFromFolder} />
                <span className="text-sm">Infer label from folder names</span>
              </div>
              {!inferLabelFromFolder && (
                <>
                  <Label>Apply single label to all</Label>
                  <Input value={applyLabel} onChange={(e)=>setApplyLabel(e.target.value)} placeholder="e.g., invoice" />
                </>
              )}
            </div>

            {/* Splits */}
            <div className="rounded-xl border p-3 space-y-3">
              <div className="text-sm font-medium flex items-center gap-2">📊 Train/Val/Test</div>
              <div className="grid grid-cols-3 gap-2">
                <div><Label>Train %</Label><Input type="number" value={splitTrain} onChange={(e)=>setSplitTrain(Number(e.target.value)||0)} /></div>
                <div><Label>Val %</Label><Input type="number" value={splitVal} onChange={(e)=>setSplitVal(Number(e.target.value)||0)} /></div>
                <div><Label>Test %</Label><Input type="number" value={splitTest} onChange={(e)=>setSplitTest(Number(e.target.value)||0)} /></div>
              </div>
              <div className={`text-xs ${splitsTotal===100 ? "text-gray-600" : "text-red-600"}`}>Total: {splitsTotal}% (must be 100%)</div>
            </div>

            {/* Options */}
            <div className="rounded-xl border p-3 space-y-3">
              <div className="text-sm font-medium">Options</div>
              <div className="grid grid-cols-2 gap-2">
                <div className="flex items-center gap-2">
                  <Switch checked={dedupe} onCheckedChange={setDedupe} />
                  <span className="text-sm">Dedupe by content hash</span>
                </div>
                <div className="flex items-center gap-2">
                  <Switch checked={normalizeImages} onCheckedChange={setNormalizeImages} />
                  <span className="text-sm">Normalize images</span>
                </div>
                <div className="flex items-center gap-2">
                  <Switch checked={tokenizeText} onCheckedChange={setTokenizeText} />
                  <span className="text-sm">Tokenize text</span>
                </div>
                <div>
                  <Label>Import Mode</Label>
                  <Select value={mode} onValueChange={(v)=>setMode(v as any)}>
                    <SelectTrigger><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="link">Link (zero-copy)</SelectItem>
                      <SelectItem value="copy">Copy into dataset</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
            </div>
          </div>

          <SheetFooter className="flex items-center justify-between gap-2">
            <div className="text-xs text-gray-600">
              {selectedCids.size} files selected
            </div>
            <div className="flex gap-2">
              <Button onClick={() => setDsOpen(false)} className="bg-gray-600 hover:bg-gray-700">
                Cancel
              </Button>
              <Button
                disabled={(useNewDataset && !dsId) || (!useNewDataset && !dsId) || splitsTotal !== 100 || selectedCids.size===0 || importingDs}
                onClick={async () => {
                  if (!dsId) return alert("Select or create a dataset first.");
                  setImportingDs(true);
                  const payload: DatasetImportOpts = {
                    datasetId: dsId,
                    cids: Array.from(selectedCids),
                    mode,
                    inferLabelFromFolder,
                    applyLabel: inferLabelFromFolder ? undefined : (applyLabel || undefined),
                    labels: dsLabels.split(",").map(s=>s.trim()).filter(Boolean),
                    splits: { train: splitTrain, val: splitVal, test: splitTest },
                    dedupe,
                    normalizeImages,
                    tokenizeText,
                    description: dsDesc || undefined,
                    license: dsVisibility,
                  };
                  const res = await apiImportToNeural(payload);
                  setImportingDs(false);
                  if (res.ok) {
                    alert(`✅ Imported ${res.imported} items to dataset.`);
                    setDsOpen(false);
                    clearSelection();
                  } else {
                    alert("❌ Import failed.");
                  }
                }}
                className="bg-green-600 hover:bg-green-700"
              >
                {importingDs ? "Importing…" : "Import"}
              </Button>
            </div>
          </SheetFooter>
        </SheetContent>
      </Sheet>

      {/* Share Modal */}
      {shareModalOpen && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-md">
            <h3 className="text-lg font-semibold mb-4">Create Share Link</h3>
            <div className="space-y-3">
              <label className="text-sm font-medium">Expires in (days)</label>
              <Input 
                type="number" 
                value={shareDays} 
                onChange={(e) => setShareDays(Math.max(1, Number(e.target.value) || 1))} 
              />
              <Button onClick={() => { makeShareLink(); }}>
                Create Link
              </Button>
              {shareUrl && (
                <div className="rounded-md border p-2">
                  <div className="text-xs text-gray-600 mb-1">URL</div>
                  <div className="font-mono text-sm break-all">{shareUrl}</div>
                </div>
              )}
            </div>
            <div className="flex gap-2 mt-4">
              <Button onClick={() => setShareModalOpen(false)} className="bg-gray-600 hover:bg-gray-700">
                Close
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
