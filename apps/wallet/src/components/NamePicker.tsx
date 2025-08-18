import { useEffect, useMemo, useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

type NamingType = "domains" | "handles";
type AvailabilityState = "idle" | "checking" | "available" | "taken" | "premium" | "invalid" | "reserved";

interface Availability {
  state: AvailabilityState;
  price?: number; // in IPN
  tld?: string;
  premium?: boolean;
  suggestions?: string[];
  message?: string;
}

// Mock API functions
async function fetchOptions() {
  // Simulate network delay
  await new Promise(resolve => setTimeout(resolve, 200));
  return {
    tlds: [
      // 1-letter (rare, strong branding)
      ".a", ".b", ".c", ".d", ".e", ".f", ".g", ".h", ".j", ".k", ".l", ".n", ".o", ".p", ".q", ".r", ".t", ".u", ".v", ".w", ".x", ".y", ".z",
      
      // 2-letter (avoid ISO country codes)
      ".aa", ".aq", ".aw", ".bx", ".cq", ".cy", ".dx", ".eh", ".fb", ".fy", ".gx", ".ii", ".iw", ".jq", ".kx", ".lq", ".mq", ".ns", ".oa", ".pb", ".qc", ".qx", ".rr", ".sx", ".ti", ".uq", ".vb", ".wc", ".ww", ".xy", ".yq", ".zz",
      
      // 3-letter (tech and finance oriented)
      ".dlt", ".dag", ".aii", ".m2m", ".iot", ".def", ".dex", ".dht", ".vmn", ".nft", ".hsh", ".ztk", ".zkp", ".stg", ".bft", ".lpk", ".p2p", ".sig", ".ecd", ".edg",
      
      // 3-letter (general use)
      ".abc", ".app", ".arc", ".bot", ".bio", ".box", ".dao", ".eco", ".eng", ".fin", ".fyi", ".hub", ".key", ".lab", ".log", ".map", ".mlt", ".new", ".pay", ".pro", ".qid", ".qos", ".run", ".sdk", ".sec", ".sup", ".sys", ".tap", ".trx", ".uid",
      
      // 4-letter (flexible + brand-style)
      ".dapp", ".edge", ".grid", ".core", ".time", ".hash", ".node", ".link", ".fund", ".data", ".file", ".home", ".life", ".open", ".safe", ".stor", ".virt", ".work", ".zone", ".unit",
      
      // Core IPPAN TLDs
      ".ipn", ".ai", ".iot", ".m", ".fin", ".dao"
    ]
  };
}

async function checkAvailability(params: { type: NamingType; name: string; tld?: string }): Promise<Availability> {
  await new Promise(resolve => setTimeout(resolve, 300));
  
  const { type, name, tld } = params;
  const fullName = tld ? `${name}${tld}` : name;
  
  // Mock availability logic
  if (!name || name.length < 2) {
    return { state: "invalid", message: "Name too short" };
  }
  
  if (name.includes("test") || name.includes("example")) {
    return { 
      state: "taken", 
      suggestions: [`${name}2`, `${name}-new`, `${name}2024`],
      message: "Name is already taken"
    };
  }
  
  if (name.length <= 3) {
    return { 
      state: "premium", 
      price: 2.0,
      premium: true,
      message: "Short names are premium"
    };
  }
  
  if (name.includes("ai") || name.includes("crypto")) {
    return { 
      state: "premium", 
      price: 1.5,
      premium: true,
      message: "AI/crypto terms are premium"
    };
  }
  
  // Standard pricing based on TLD
  let price = 0.2; // Year 1 base price
  
  // Premium multipliers based on TLD type
  if (tld === ".ai" || tld === ".m") {
    price *= 10; // Premium domains
  } else if (tld === ".iot") {
    price *= 2; // IoT domains
  } else if (tld && tld.length === 1) {
    // 1-letter TLDs (e.g., .a, .b, .c) - ultra premium
    price *= 20;
  } else if (tld && tld.length === 2) {
    // 2-letter TLDs (e.g., .aa, .xy) - very premium
    price *= 15;
  } else if (["dlt", "dag", "aii", "m2m", "nft", "zkp", "dao", "def", "dex"].includes(tld?.substring(1) || "")) {
    // Tech/crypto terms - premium
    price *= 5;
  }
  
  return { 
    state: "available", 
    price,
    tld: tld || ".ipn"
  };
}

async function reserveName(body: { type: NamingType; name: string; tld?: string }) {
  await new Promise(resolve => setTimeout(resolve, 500));
  return {
    hold_id: `hold_${Date.now()}`,
    expires_at: new Date(Date.now() + 15000).toISOString() // 15 seconds
  };
}

async function registerName(body: { hold_id: string; signature: string }) {
  await new Promise(resolve => setTimeout(resolve, 1000));
  return {
    txid: `tx_${Date.now()}`,
    hashtimer: `ht_${Date.now()}`
  };
}

export default function NamePicker({ type }: { type: NamingType }) {
  const qc = useQueryClient();
  const [raw, setRaw] = useState("");
  const [tld, setTld] = useState<string | undefined>(undefined);
  const [debounced, setDebounced] = useState("");
  const [hold, setHold] = useState<{ id: string; expires_at: string } | null>(null);

  // Debounce input
  useEffect(() => {
    const h = setTimeout(() => setDebounced(raw.trim()), 300);
    return () => clearTimeout(h);
  }, [raw]);

  const normalized = useMemo(() => {
    if (type === "handles") {
      // force @ and .ipn if omitted
      let v = raw.trim();
      if (!v.startsWith("@")) v = "@" + v;
      if (!v.endsWith(".ipn")) v = v + ".ipn";
      return v.toLowerCase();
    }
    return raw.toLowerCase();
  }, [raw, type]);

  const { data: options } = useQuery({ queryKey: ["namingOptions"], queryFn: fetchOptions });

  const { data: avail, isFetching } = useQuery({
    queryKey: ["availability", type, debounced, tld],
    queryFn: () => checkAvailability({ type, name: debounced, tld }),
    enabled: debounced.length > 0
  });

  const reserveMut = useMutation({ 
    mutationFn: reserveName, 
    onSuccess: (d) => setHold({ id: d.hold_id, expires_at: d.expires_at }) 
  });
  
  const registerMut = useMutation({ 
    mutationFn: registerName, 
    onSuccess: ({ txid }) => {
      setHold(null);
      qc.invalidateQueries({ queryKey: ["availability", type, debounced, tld] });
      alert(`Registered! TX: ${txid}`);
    }
  });

  const canReserve = avail?.state === "available" || avail?.state === "premium";
  const canRegister = hold && new Date(hold.expires_at).getTime() > Date.now();

  return (
    <div className="space-y-3">
      <div className="flex gap-2 items-center">
        <input
          className="w-full rounded-xl border px-3 py-2"
          placeholder={type === "domains" ? "e.g. acme" : "e.g. @alice.ipn"}
          value={raw}
          onChange={(e) => setRaw(e.target.value)}
        />
        {type === "domains" && (
          <select className="rounded-xl border px-2 py-2" value={tld ?? ""} onChange={(e) => setTld(e.target.value || undefined)}>
            <option value="">.ipn</option>
            {options?.tlds?.map((t: string) => <option key={t} value={t}>{t}</option>)}
          </select>
        )}
      </div>

      {/* Status */}
      <div className="rounded-xl border p-3">
        {(!debounced || avail?.state === "idle") && <p>Start typing to check availability…</p>}
        {isFetching && <p>Checking availability…</p>}

        {avail && !isFetching && (
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-sm">Status:</span>
              <span className={`text-sm font-medium ${avail.state === 'available' ? 'text-green-600' : avail.state === 'taken' ? 'text-red-600' : 'text-orange-600'}`}>
                {avail.state.toUpperCase()}
              </span>
            </div>

            {avail.price != null && (
              <div className="flex items-center justify-between">
                <span className="text-sm">Price:</span>
                <span className="text-sm font-semibold">{avail.price} IPN → Global Fund</span>
              </div>
            )}

            {avail.message && (
              <div className="text-sm text-gray-600">{avail.message}</div>
            )}

            {avail.state === "taken" && avail.suggestions?.length ? (
              <div>
                <div className="text-sm mb-1">Suggestions:</div>
                <div className="flex flex-wrap gap-2">
                  {avail.suggestions.map((s) => (
                    <button key={s} className="rounded-full border px-3 py-1 text-sm" onClick={() => setRaw(s)}>
                      {s}
                    </button>
                  ))}
                </div>
              </div>
            ) : null}

            {/* Actions */}
            <div className="flex gap-2">
              <button
                className="rounded-xl px-3 py-2 border disabled:opacity-50"
                disabled={!canReserve || reserveMut.isPending}
                onClick={() => reserveMut.mutate({ type, name: debounced, tld })}
              >
                {reserveMut.isPending ? "Reserving…" : "Reserve (15s)"}
              </button>

              <button
                className="rounded-xl px-3 py-2 border disabled:opacity-50"
                disabled={!canRegister || registerMut.isPending}
                onClick={async () => {
                  // Mock signature from Wallet
                  const signature = `mock_sig_${Date.now()}`;
                  registerMut.mutate({ hold_id: hold!.id, signature });
                }}
              >
                {registerMut.isPending ? "Registering…" : "Register"}
              </button>

              {hold && (
                <div className="text-xs opacity-70 self-center">
                  Hold expires: {new Date(hold.expires_at).toLocaleTimeString()}
                </div>
              )}
            </div>
          </div>
        )}
      </div>

      <div className="text-xs opacity-70">
        By registering, you agree to publish the domain/handle record on-chain. Fees flow to the Global Fund. Renewal reminders will appear in Wallet.
      </div>
    </div>
  );
}
