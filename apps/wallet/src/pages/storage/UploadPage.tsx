import React, { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";

/** -----------------------------
 * Types & helpers
 * ------------------------------ */

type AccessPolicy =
  | { type: "public" }                           // key published openly
  | { type: "private" }                          // only owner
  | { type: "paid"; priceIPN: number };          // pay-to-unlock

type LeaseOption = {
  years: number;          // 1, 2, 3...
  autoRenew: boolean;
};

type FeeEstimate = {
  storageFeeIPN: number;  // bytes & lease based
  publishFeeIPN: number;  // micro-fee / tx fee, etc.
  totalIPN: number;
};

type InitResponse = {
  uploadId: string;
  shardSize: number;
  shards: { id: string; index: number; size: number }[];
  hashtimerHint: string;
  manifestPreview: {
    contentId: string;
    redundancy: number;
    class: "privacy" | "compliance";
  };
};

type CommitResponse = {
  hashtimer: string;
  contentId: string;
  shareUrl?: string; // optional vanity if linked to a domain/handle
};

function formatIPN(n: number) {
  return n.toLocaleString(undefined, { minimumFractionDigits: 6, maximumFractionDigits: 6 });
}

function bytesHuman(n: number) {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let i = -1;
  do {
    n = n / 1024;
    i++;
  } while (n >= 1024 && i < units.length - 1);
  return `${n.toFixed(2)} ${units[i]}`;
}

/** Mock encryption using WebCrypto (AES-GCM).
 *  Replace with your actual encrypt-on-chunk logic.
 */
async function encryptChunkGCM(
  chunk: ArrayBuffer,
  key: CryptoKey
): Promise<{ data: ArrayBuffer; iv: Uint8Array }> {
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const data = await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, chunk);
  return { data, iv };
}

async function generateKey(): Promise<CryptoKey> {
  return crypto.subtle.generateKey({ name: "AES-GCM", length: 256 }, true, ["encrypt", "decrypt"]);
}

async function exportKeyJwk(key: CryptoKey) {
  return await crypto.subtle.exportKey("jwk", key);
}

/** -----------------------------
 * Mock API (replace with real)
 * ------------------------------ */
async function apiInitUpload(params: {
  sizeBytes: number;
  shardSize: number;
  redundancy: number;
  class_: "privacy" | "compliance";
  leaseYears: number;
  linkTo?: string | null;
  access: AccessPolicy;
}): Promise<{ estimate: FeeEstimate; init: InitResponse }> {
  // Simulate a fee model: storage fee ~= bytes * years * 1e-9 IPN, publish fee flat
  const storageFeeIPN = (params.sizeBytes / 1_000_000) * params.leaseYears * 0.000001;
  const publishFeeIPN = 0.000010; // 10 micro IPN placeholder
  const totalIPN = storageFeeIPN + publishFeeIPN;

  const shards = Math.ceil(params.sizeBytes / params.shardSize);
  const arr = Array.from({ length: shards }, (_, i) => ({
    id: `shard_${i}_${Math.random().toString(16).slice(2)}`,
    index: i,
    size:
      i === shards - 1
        ? params.sizeBytes - (shards - 1) * params.shardSize
        : params.shardSize,
  }));

  return new Promise((resolve) =>
    setTimeout(
      () =>
        resolve({
          estimate: { storageFeeIPN, publishFeeIPN, totalIPN },
          init: {
            uploadId: "upl_" + Math.random().toString(36).slice(2),
            shardSize: params.shardSize,
            shards: arr,
            hashtimerHint: "HT-PREVIEW-" + Date.now(),
            manifestPreview: {
              contentId: "CID-" + Math.random().toString(16).slice(2),
              redundancy: params.redundancy,
              class: params.class_,
            },
          },
        }),
      400
    )
  );
}

async function apiUploadShard(uploadId: string, shardId: string, payload: ArrayBuffer) {
  // Simulate per-shard upload latency
  await new Promise((r) => setTimeout(r, 120));
  return { ok: true };
}

async function apiCommitUpload(uploadId: string, manifestSig: string): Promise<CommitResponse> {
  return new Promise((resolve) =>
    setTimeout(
      () =>
        resolve({
          hashtimer: "0x" + crypto.getRandomValues(new Uint8Array(32)).reduce((s, b) => s + b.toString(16).padStart(2, "0"), ""),
          contentId: "CID-" + Math.random().toString(16).slice(2),
          shareUrl: undefined, // fill if linking to a domain/handle
        }),
      400
    )
  );
}

/** -----------------------------
 * Local reducer for progress modal
 * ------------------------------ */
type ProgressState =
  | { step: "idle" }
  | { step: "encrypting"; current: number; total: number }
  | { step: "uploading"; current: number; total: number }
  | { step: "committing" }
  | { step: "done"; result: CommitResponse }
  | { step: "error"; message: string };

type Action =
  | { type: "RESET" }
  | { type: "ENCRYPTING"; current: number; total: number }
  | { type: "UPLOADING"; current: number; total: number }
  | { type: "COMMITTING" }
  | { type: "DONE"; result: CommitResponse }
  | { type: "ERROR"; message: string };

function progressReducer(state: ProgressState, action: Action): ProgressState {
  switch (action.type) {
    case "RESET":
      return { step: "idle" };
    case "ENCRYPTING":
      return { step: "encrypting", current: action.current, total: action.total };
    case "UPLOADING":
      return { step: "uploading", current: action.current, total: action.total };
    case "COMMITTING":
      return { step: "committing" };
    case "DONE":
      return { step: "done", result: action.result };
    case "ERROR":
      return { step: "error", message: action.message };
    default:
      return state;
  }
}

/** -----------------------------
 * Subcomponents
 * ------------------------------ */

function FileDropZone({
  onFiles,
  disabled,
}: {
  onFiles: (files: File[]) => void;
  disabled?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);

  const onDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      if (disabled) return;
      const files = Array.from(e.dataTransfer.files || []).filter((f) => f.size > 0);
      onFiles(files);
    },
    [onFiles, disabled]
  );

  return (
    <div
      ref={ref}
      onDragOver={(e) => e.preventDefault()}
      onDrop={onDrop}
      className={`border-2 border-dashed rounded-2xl p-8 text-center ${
        disabled ? "opacity-60" : "hover:bg-gray-50"
      }`}
    >
      <div className="text-lg font-semibold">Drag & drop files here</div>
      <div className="text-sm text-gray-600 mt-1">or</div>
      <label className="inline-block mt-3">
        <input
          type="file"
          multiple
          className="hidden"
          onChange={(e) => {
            const files = Array.from(e.target.files || []).filter((f) => f.size > 0);
            onFiles(files);
          }}
          disabled={disabled}
        />
        <span className="px-4 py-2 rounded-xl border cursor-pointer">Choose files</span>
      </label>
    </div>
  );
}

function EncryptionSettings({
  encrypt,
  setEncrypt,
  shardSize,
  setShardSize,
  redundancy,
  setRedundancy,
  complianceClass,
  setComplianceClass,
}: {
  encrypt: boolean;
  setEncrypt: (v: boolean) => void;
  shardSize: number;
  setShardSize: (n: number) => void;
  redundancy: number;
  setRedundancy: (n: number) => void;
  complianceClass: boolean;
  setComplianceClass: (b: boolean) => void;
}) {
  return (
    <div className="grid md:grid-cols-2 gap-4">
      <label className="flex items-center gap-3">
        <input type="checkbox" checked={encrypt} onChange={(e) => setEncrypt(e.target.checked)} />
        <span className="font-medium">Encrypt before publishing</span>
      </label>

      <label className="flex items-center gap-3">
        <input
          type="checkbox"
          checked={complianceClass}
          onChange={(e) => setComplianceClass(e.target.checked)}
        />
        <span>Compliance class (chunk IDs = hash(plaintext))</span>
      </label>

      <label className="flex items-center gap-3">
        <span className="w-40">Shard size</span>
        <select
          value={shardSize}
          onChange={(e) => setShardSize(parseInt(e.target.value))}
          className="border rounded-xl px-3 py-2"
        >
          <option value={1 << 20}>1 MB</option>
          <option value={5 << 20}>5 MB</option>
          <option value={10 << 20}>10 MB</option>
        </select>
      </label>

      <label className="flex items-center gap-3">
        <span className="w-40">Redundancy</span>
        <select
          value={redundancy}
          onChange={(e) => setRedundancy(parseInt(e.target.value))}
          className="border rounded-xl px-3 py-2"
        >
          <option value={1}>1×</option>
          <option value={2}>2×</option>
          <option value={3}>3×</option>
        </select>
      </label>
    </div>
  );
}

function AccessSettings({
  access,
  setAccess,
  linkTo,
  setLinkTo,
}: {
  access: AccessPolicy;
  setAccess: (p: AccessPolicy) => void;
  linkTo: string;
  setLinkTo: (s: string) => void;
}) {
  return (
    <div className="grid md:grid-cols-2 gap-4">
      <div className="space-y-2">
        <div className="font-semibold">Access policy</div>
        <div className="flex flex-col gap-2">
          <label className="flex items-center gap-2">
            <input
              type="radio"
              name="ap"
              checked={access.type === "public"}
              onChange={() => setAccess({ type: "public" })}
            />
            <span>Public (key published openly)</span>
          </label>
          <label className="flex items-center gap-2">
            <input
              type="radio"
              name="ap"
              checked={access.type === "private"}
              onChange={() => setAccess({ type: "private" })}
            />
            <span>Private (only you)</span>
          </label>
          <label className="flex items-center gap-2">
            <input
              type="radio"
              name="ap"
              checked={access.type === "paid"}
              onChange={() => setAccess({ type: "paid", priceIPN: 0.01 })}
            />
            <span>Paid unlock</span>
            {access.type === "paid" && (
              <input
                type="number"
                value={access.priceIPN}
                step={0.001}
                min={0}
                onChange={(e) =>
                  setAccess({ type: "paid", priceIPN: parseFloat(e.target.value || "0") })
                }
                className="ml-2 w-28 border rounded-xl px-2 py-1"
                placeholder="Price (IPN)"
              />
            )}
          </label>
        </div>
      </div>

      <div className="space-y-2">
        <div className="font-semibold">Publish TXT / Link to Domain or Handle</div>
        <input
          className="w-full border rounded-xl px-3 py-2"
          placeholder="e.g., @alice.ipn or site.ipn"
          value={linkTo}
          onChange={(e) => setLinkTo(e.target.value)}
        />
        <div className="text-xs text-gray-500">
          Optional. Creates a discoverable pointer in the IPPAN naming layer.
        </div>
      </div>
    </div>
  );
}

function LeaseOptions({
  lease,
  setLease,
  estimatedFee,
}: {
  lease: LeaseOption;
  setLease: (l: LeaseOption) => void;
  estimatedFee?: FeeEstimate | null;
}) {
  return (
    <div className="grid md:grid-cols-2 gap-4">
      <label className="flex items-center gap-3">
        <span className="w-40">Lease duration</span>
        <select
          value={lease.years}
          onChange={(e) => setLease({ ...lease, years: parseInt(e.target.value) })}
          className="border rounded-xl px-3 py-2"
        >
          <option value={1}>1 year</option>
          <option value={2}>2 years</option>
          <option value={3}>3 years</option>
        </select>
      </label>
      <label className="flex items-center gap-3">
        <input
          type="checkbox"
          checked={lease.autoRenew}
          onChange={(e) => setLease({ ...lease, autoRenew: e.target.checked })}
        />
        <span>Auto-renew lease (requires wallet balance)</span>
      </label>

      <div className="md:col-span-2">
        <div className="rounded-xl border p-3">
          <div className="font-semibold mb-2">Estimated cost</div>
          {estimatedFee ? (
            <div className="grid grid-cols-2 md:grid-cols-4 gap-2 text-sm">
              <div>
                <div className="text-gray-500">Storage</div>
                <div className="font-medium">{formatIPN(estimatedFee.storageFeeIPN)} IPN</div>
              </div>
              <div>
                <div className="text-gray-500">Publish / Tx</div>
                <div className="font-medium">{formatIPN(estimatedFee.publishFeeIPN)} IPN</div>
              </div>
              <div>
                <div className="text-gray-500">Total</div>
                <div className="font-semibold">{formatIPN(estimatedFee.totalIPN)} IPN</div>
              </div>
              <div className="text-gray-500">
                All fees flow to the Global Fund (auto-distributed).
              </div>
            </div>
          ) : (
            <div className="text-sm text-gray-500">Select files to see fee estimate.</div>
          )}
        </div>
      </div>
    </div>
  );
}

function WalletConfirm({
  disabled,
  onPublish,
}: {
  disabled: boolean;
  onPublish: () => void;
}) {
  return (
    <div className="flex items-center justify-between">
      <div className="text-xs text-gray-500">
        By publishing, you accept that encryption keys are your responsibility. If lost, data is
        unrecoverable.
      </div>
      <button
        disabled={disabled}
        onClick={onPublish}
        className={`px-4 py-2 rounded-xl border ${disabled ? "opacity-50" : "hover:bg-gray-50"}`}
      >
        Publish File
      </button>
    </div>
  );
}

function ProgressModal({
  state,
  onClose,
}: {
  state: ProgressState;
  onClose: () => void;
}) {
  const percent =
    state.step === "encrypting" || state.step === "uploading"
      ? Math.round((state.current / Math.max(state.total, 1)) * 100)
      : state.step === "committing"
      ? 95
      : state.step === "done"
      ? 100
      : 0;

  if (state.step === "idle") return null;

  return (
    <div className="fixed inset-0 bg-black/30 flex items-center justify-center z-50">
      <div className="bg-white rounded-2xl shadow-xl max-w-lg w-full p-6">
        <div className="flex items-center justify-between mb-3">
          <div className="text-lg font-semibold">Publishing to IPPAN</div>
          <button onClick={onClose} className="text-sm text-gray-500 hover:text-gray-700">Close</button>
        </div>

        <div className="w-full bg-gray-100 rounded-full h-2 mb-3 overflow-hidden">
          <div
            className="bg-gray-700 h-2 rounded-full transition-all"
            style={{ width: `${percent}%` }}
          />
        </div>

        {state.step === "encrypting" && (
          <div className="text-sm">Encrypting chunks… {state.current}/{state.total}</div>
        )}
        {state.step === "uploading" && (
          <div className="text-sm">Uploading shards… {state.current}/{state.total}</div>
        )}
        {state.step === "committing" && <div className="text-sm">Committing manifest…</div>}
        {state.step === "error" && (
          <div className="text-sm text-red-600">Error: {state.message}</div>
        )}
        {state.step === "done" && (
          <div className="space-y-2">
            <div className="text-sm text-green-700">Done! Your file is now published.</div>
            <div className="rounded-xl border p-3 text-sm">
              <div><span className="text-gray-500">HashTimer:</span> <span className="font-mono break-all">{state.result.hashtimer}</span></div>
              <div><span className="text-gray-500">Content ID:</span> <span className="font-mono break-all">{state.result.contentId}</span></div>
              {state.result.shareUrl && (
                <div><span className="text-gray-500">Share URL:</span> <span className="font-mono break-all">{state.result.shareUrl}</span></div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

/** -----------------------------
 * Main page
 * ------------------------------ */

export default function UploadPage() {
  const [files, setFiles] = useState<File[]>([]);
  const [encrypt, setEncrypt] = useState(true);
  const [complianceClass, setComplianceClass] = useState(false);
  const [shardSize, setShardSize] = useState(5 << 20); // 5 MB default
  const [redundancy, setRedundancy] = useState(2);
  const [access, setAccess] = useState<AccessPolicy>({ type: "public" });
  const [linkTo, setLinkTo] = useState("");
  const [lease, setLease] = useState<LeaseOption>({ years: 1, autoRenew: false });

  const [estimate, setEstimate] = useState<FeeEstimate | null>(null);
  const [init, setInit] = useState<InitResponse | null>(null);
  const [modal, dispatch] = useReducer(progressReducer, { step: "idle" });

  const totalBytes = useMemo(
    () => files.reduce((acc, f) => acc + f.size, 0),
    [files]
  );

  // Estimate fees when parameters change
  useEffect(() => {
    let cancelled = false;
    if (!totalBytes) {
      setEstimate(null);
      return;
    }
    (async () => {
      const { estimate } = await apiInitUpload({
        sizeBytes: totalBytes,
        shardSize,
        redundancy,
        class_: complianceClass ? "compliance" : "privacy",
        leaseYears: lease.years,
        linkTo: linkTo || null,
        access,
      });
      if (!cancelled) setEstimate(estimate);
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [totalBytes, shardSize, redundancy, complianceClass, lease.years, linkTo, JSON.stringify(access)]);

  // Start publishing flow
  const onPublish = useCallback(async () => {
    try {
      if (!totalBytes || files.length === 0) return;

      // 1) Init with same params used for estimate (so backend returns shard plan)
      const initResp = await apiInitUpload({
        sizeBytes: totalBytes,
        shardSize,
        redundancy,
        class_: complianceClass ? "compliance" : "privacy",
        leaseYears: lease.years,
        linkTo: linkTo || null,
        access,
      });
      setInit(initResp.init);

      // 2) Generate key if encrypting
      let cryptoKey: CryptoKey | null = null;
      if (encrypt) cryptoKey = await generateKey();
      const jwk = cryptoKey ? await exportKeyJwk(cryptoKey) : null;

      // 3) Slice file(s) into shards, encrypt, upload
      //    For simplicity, we concatenate files in order; production should build a manifest per-file.
      const bigBuffer = await (async () => {
        const bufs: ArrayBuffer[] = [];
        for (const f of files) bufs.push(await f.arrayBuffer());
        // concat
        const total = bufs.reduce((a, b) => a + b.byteLength, 0);
        const out = new Uint8Array(total);
        let off = 0;
        for (const buf of bufs) {
          out.set(new Uint8Array(buf), off);
          off += buf.byteLength;
        }
        return out.buffer;
      })();

      // Encrypt & upload per shard
      const totalShards = Math.ceil(bigBuffer.byteLength / shardSize);
      const view = new Uint8Array(bigBuffer);
      let offset = 0;

      for (let i = 0; i < totalShards; i++) {
        const end = Math.min(offset + shardSize, view.byteLength);
        const chunk = view.slice(offset, end).buffer;
        offset = end;

        // Encrypt progress
        dispatch({ type: "ENCRYPTING", current: i + 1, total: totalShards });

        let payload: ArrayBuffer = chunk;
        if (encrypt && cryptoKey) {
          const enc = await encryptChunkGCM(chunk, cryptoKey);
          // In a real implementation, you'd also upload the IV & tag; here we just send ciphertext.
          payload = enc.data;
        }

        // Upload progress
        dispatch({ type: "UPLOADING", current: i + 1, total: totalShards });
        const shardId = initResp.init.shards[i]?.id ?? `local_${i}`;
        await apiUploadShard(initResp.init.uploadId, shardId, payload);
      }

      // 4) Commit manifest (signing via wallet)
      dispatch({ type: "COMMITTING" });
      const manifestToSign = JSON.stringify({
        uploadId: initResp.init.uploadId,
        hashtimerHint: initResp.init.hashtimerHint,
        contentId: initResp.init.manifestPreview.contentId,
        redundancy,
        class: complianceClass ? "compliance" : "privacy",
        linkTo: linkTo || null,
        access,
        keyJwk: jwk, // NOTE: if access=public, key can be published; otherwise store off-chain
      });

      // Replace with your wallet bridge:
      // @ts-ignore
      const signature: string =
        (window.wallet && (await window.wallet.sign(manifestToSign))) ||
        "SIMULATED_SIGNATURE";

      const committed = await apiCommitUpload(initResp.init.uploadId, signature);
      dispatch({ type: "DONE", result: committed });
    } catch (e: any) {
      dispatch({ type: "ERROR", message: e?.message || "Unknown error" });
    }
  }, [
    files,
    totalBytes,
    shardSize,
    redundancy,
    complianceClass,
    lease.years,
    linkTo,
    access,
    encrypt,
  ]);

  const disabled = !files.length || !estimate;

  return (
    <div className="max-w-4xl mx-auto p-4 md:p-6 space-y-6">
      <header className="space-y-1">
        <h1 className="text-2xl font-bold">Upload to IPPAN Storage</h1>
        <p className="text-sm text-gray-600">
          Files are encrypted, sharded, and published across IPNWorkers. You control access keys &
          lease policy.
        </p>
      </header>

      {/* File list / dropzone */}
      <section className="space-y-3">
        <FileDropZone onFiles={setFiles} disabled={modal.step !== "idle"} />
        {files.length > 0 && (
          <div className="rounded-2xl border p-3">
            <div className="font-semibold mb-2">Selected files</div>
            <ul className="divide-y">
              {files.map((f, i) => (
                <li key={i} className="py-2 flex items-center justify-between">
                  <div className="truncate">
                    <span className="font-medium">{f.name}</span>
                    <span className="text-gray-500 ml-2 text-sm">{bytesHuman(f.size)}</span>
                  </div>
                  <button
                    className="text-sm text-red-600 hover:underline"
                    onClick={() => setFiles(files.filter((_, j) => j !== i))}
                  >
                    remove
                  </button>
                </li>
              ))}
            </ul>
            <div className="text-sm text-gray-600 mt-2">
              Total: <span className="font-medium">{bytesHuman(totalBytes)}</span>
            </div>
          </div>
        )}
      </section>

      {/* Settings */}
      <section className="rounded-2xl border p-4 space-y-4">
        <h2 className="text-lg font-semibold">Encryption & Sharding</h2>
        <EncryptionSettings
          encrypt={encrypt}
          setEncrypt={setEncrypt}
          shardSize={shardSize}
          setShardSize={setShardSize}
          redundancy={redundancy}
          setRedundancy={setRedundancy}
          complianceClass={complianceClass}
          setComplianceClass={setComplianceClass}
        />
      </section>

      <section className="rounded-2xl border p-4 space-y-4">
        <h2 className="text-lg font-semibold">Access & Metadata</h2>
        <AccessSettings access={access} setAccess={setAccess} linkTo={linkTo} setLinkTo={setLinkTo} />
      </section>

      <section className="rounded-2xl border p-4 space-y-4">
        <h2 className="text-lg font-semibold">Lease & Fees</h2>
        <LeaseOptions lease={lease} setLease={setLease} estimatedFee={estimate} />
      </section>

      <WalletConfirm disabled={disabled || modal.step !== "idle"} onPublish={() => onPublish()} />

      <ProgressModal
        state={modal}
        onClose={() => {
          if (modal.step === "done" || modal.step === "error") {
            // allow closing; reset to start new upload
            setFiles([]);
            setEstimate(null);
            setInit(null);
            dispatch({ type: "RESET" });
          }
        }}
      />

      {/* Preview card */}
      {init && (
        <section className="rounded-2xl border p-4 space-y-2">
          <div className="font-semibold">Manifest preview</div>
          <div className="text-sm grid md:grid-cols-2 gap-2">
            <div>
              <div className="text-gray-500">Content ID</div>
              <div className="font-mono break-all">{init.manifestPreview.contentId}</div>
            </div>
            <div>
              <div className="text-gray-500">HashTimer (hint)</div>
              <div className="font-mono break-all">{init.hashtimerHint}</div>
            </div>
            <div>
              <div className="text-gray-500">Shards</div>
              <div className="font-medium">{init.shards.length} × {bytesHuman(init.shardSize)}</div>
            </div>
            <div>
              <div className="text-gray-500">Class / Redundancy</div>
              <div className="font-medium">
                {init.manifestPreview.class} / {init.manifestPreview.redundancy}×
              </div>
            </div>
          </div>
        </section>
      )}
    </div>
  );
}
