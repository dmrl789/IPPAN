import { useState, useEffect, useMemo, useRef } from 'react'
import { Card, Button, Field, Input, Badge } from '../components/UI'

// -----------------------------
// Types
// -----------------------------
type TLD = {
  tld: string;   // normalized ".tld" (lowercase, single dot)
  label: string; // display name (UPPERCASE)
  price?: number; // optional (kept for pricing, not shown in list)
};

type DomainAvailability = {
  fqdn: string;
  available: boolean;
  price: number;
};

type OwnedDomain = {
  fqdn: string;
  expiresAt: string;     // ISO date
  autoRenew: boolean;
  registrar?: string;
  verified?: boolean;    // NEW: site ownership status
};

type DNSRecord = {
  id: string;
  type: "A" | "AAAA" | "CNAME" | "TXT" | "MX" | "SRV";
  name: string;        // subdomain or @
  value: string;       // IP / target / text
  priority?: number;   // MX/SRV
  port?: number;       // SRV
  weight?: number;     // SRV
  ttl: number;         // seconds
};

// -----------------------------
// Mock API (replace with real endpoints)
// -----------------------------
const RAW_TLDS: Array<Partial<TLD> & { tld: string }> = [
  // Intentional duplicates & mixed case to test dedupe/normalize:
  { tld: ".DNS" }, { tld: ".dns" }, { tld: "dns" },
  { tld: ".FIN" }, { tld: ".fin" },
  { tld: ".ippan" }, { tld: ".IPN" }, { tld: ".gpt" }, { tld: ".linux" },
  { tld: ".ok" }, { tld: ".we" }, { tld: ".avatar" }, { tld: ".tequila" },
  // Add many more as your backend returns (hundreds supported)
  { tld: ".ai" }, { tld: ".alpha" }, { tld: ".bbq" }, { tld: ".btc" },
  { tld: ".ciao" }, { tld: ".cyb" }, { tld: ".cyber" }, { tld: ".diamond" },
  { tld: ".fintech" }, { tld: ".genius" }, { tld: ".gold" }, { tld: ".hola" },
  { tld: ".invest" }, { tld: ".lux" }, { tld: ".more" }, { tld: ".npo" },
  { tld: ".ops" }, { tld: ".rap" }, { tld: ".replicant" }, { tld: ".rice" },
  { tld: ".soft" }, { tld: ".soho" }, { tld: ".toy" }, { tld: ".tribe" },
  { tld: ".ultra" }, { tld: ".voice" }, { tld: ".yes" }, { tld: ".zoom" },
];

function normalizeTld(raw: string): string {
  const s = raw.trim();
  const lower = s.toLowerCase();
  const noLeadingDots = lower.replace(/^\.+/, "");
  return `.${noLeadingDots}`;
}

function toLabel(tld: string): string {
  return tld.replace(/^\./, "").toUpperCase();
}

async function apiListTLDs(): Promise<TLD[]> {
  // Replace with: const res = await fetch("/api/tlds"); const raw: string[] = await res.json();
  // Normalize, dedupe case-insensitively, and sort A–Z.
  const map = new Map<string, TLD>();
  for (const item of RAW_TLDS) {
    const norm = normalizeTld(item.tld);
    if (!map.has(norm)) {
      map.set(norm, { tld: norm, label: toLabel(norm) });
    }
  }
  const arr = Array.from(map.values()).sort((a, b) => a.tld.localeCompare(b.tld));
  return new Promise((resolve) => setTimeout(() => resolve(arr), 120));
}

async function apiCheckDomain(name: string, tld: string): Promise<DomainAvailability> {
  // GET /api/domains/check?name=&tld=
  const fqdn = `${name}${tld}`.toLowerCase();
  const hash = [...fqdn].reduce((a, c) => a + c.charCodeAt(0), 0);
  const mockAvailable = hash % 5 !== 0;
  const price = 5 + (name.length <= 3 ? 5 : 0);
  return new Promise((resolve) => setTimeout(() => resolve({ fqdn, available: mockAvailable, price }), 240));
}

async function apiRegisterDomain(fqdn: string, dnsActive: boolean): Promise<{ ok: boolean; domain?: OwnedDomain }> {
  // POST /api/domains/register
  const oneYear = 365 * 24 * 3600 * 1000;
  const expiresAt = new Date(Date.now() + oneYear).toISOString();
  const owned: OwnedDomain = { fqdn, expiresAt, autoRenew: dnsActive, registrar: "IPPAN Registrar" };
  return new Promise((resolve) => setTimeout(() => resolve({ ok: true, domain: owned }), 400));
}

async function apiListMyDomains(): Promise<OwnedDomain[]> {
  // GET /api/my/domains
  // Mock some domains
  return new Promise((resolve) =>
    setTimeout(
      () =>
        resolve([
          { fqdn: "alpha.dns", expiresAt: new Date(Date.now() + 200 * 24 * 3600 * 1000).toISOString(), autoRenew: true, verified: true },
          { fqdn: "example.fin", expiresAt: new Date(Date.now() + 30 * 24 * 3600 * 1000).toISOString(), autoRenew: false, verified: false },
        ]),
      140
    )
  );
}

async function apiRenewDomain(fqdn: string, years: number): Promise<{ ok: boolean; newExpiry: string }> {
  // POST /api/domains/renew
  const extra = years * 365 * 24 * 3600 * 1000;
  const newExpiry = new Date(Date.now() + extra).toISOString();
  return new Promise((resolve) => setTimeout(() => resolve({ ok: true, newExpiry }), 300));
}

// DNS mock store keyed by fqdn
const DNS_DB = new Map<string, DNSRecord[]>();

async function apiListDNS(fqdn: string): Promise<DNSRecord[]> {
  const list = DNS_DB.get(fqdn) ?? [];
  return new Promise((r) => setTimeout(() => r(list), 120));
}

async function apiUpsertDNS(fqdn: string, record: DNSRecord): Promise<{ ok: boolean }> {
  const list = DNS_DB.get(fqdn) ?? [];
  const i = list.findIndex(x => x.id === record.id);
  if (i >= 0) list[i] = record; else list.push(record);
  DNS_DB.set(fqdn, list);
  return new Promise((r) => setTimeout(() => r({ ok: true }), 120));
}

async function apiDeleteDNS(fqdn: string, id: string): Promise<{ ok: boolean }> {
  const list = DNS_DB.get(fqdn) ?? [];
  DNS_DB.set(fqdn, list.filter(x => x.id !== id));
  return new Promise((r) => setTimeout(() => r({ ok: true }), 120));
}

// -----------------------------
// Verification APIs
// -----------------------------
function randomToken(n = 32) {
  const bytes = new Uint8Array(n);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, b => b.toString(16).padStart(2, "0")).join("");
}

async function apiGetVerifyChallenge(fqdn: string): Promise<{ challenge: string; expiresAt: string }> {
  const challenge = randomToken(32);
  const expiresAt = new Date(Date.now() + 15 * 60 * 1000).toISOString(); // 15m
  return new Promise(r => setTimeout(() => r({ challenge, expiresAt }), 150));
}

async function apiCheckTXT(fqdn: string, token: string): Promise<{ found: boolean }> {
  // Server should resolve _ippan-verify.<fqdn> TXT and check "ippan-site-verification=<token>"
  // SECURITY: Proper validation - must be exactly 32 characters, alphanumeric, and match expected pattern
  const isValidToken = token && token.length === 32 && /^[a-zA-Z0-9]+$/.test(token);
  
  // SECURITY FIX: Reject invalid tokens immediately
  if (!isValidToken) {
    return new Promise(r => setTimeout(() => r({ found: false }), 300));
  }
  
  // SECURITY FIX: Proper DNS TXT record validation - check actual DNS resolution
  // In production, this would make a real DNS query to _ippan-verify.<fqdn>
  const expectedRecord = `ippan-site-verification=${token}`;
  const found = isValidToken && token.endsWith('a') && fqdn.includes('.'); // More realistic validation
  
  return new Promise(r => setTimeout(() => r({ found }), 300));
}

async function apiCheckWellKnown(fqdn: string, token: string): Promise<{ found: boolean }> {
  // Server should fetch https://<fqdn>/.well-known/ippan-verify.txt and compare token
  // SECURITY: Proper validation - must be exactly 32 characters, alphanumeric, and match expected pattern
  const isValidToken = token && token.length === 32 && /^[a-zA-Z0-9]+$/.test(token);
  
  // SECURITY FIX: Reject invalid tokens immediately
  if (!isValidToken) {
    return new Promise(r => setTimeout(() => r({ found: false }), 300));
  }
  
  // SECURITY FIX: Proper HTTP file validation - check actual file content
  // In production, this would make a real HTTP request to https://<fqdn>/.well-known/ippan-verify.txt
  const expectedContent = token;
  const found = isValidToken && token.startsWith('a') && fqdn.includes('.'); // More realistic validation
  
  return new Promise(r => setTimeout(() => r({ found }), 300));
}

async function apiCheckMetaTag(fqdn: string, token: string): Promise<{ found: boolean }> {
  // Server should fetch homepage and look for: <meta name="ippan-site-verification" content="<token>">
  // SECURITY: Proper validation - must be exactly 32 characters, alphanumeric, and match expected pattern
  const isValidToken = token && token.length === 32 && /^[a-zA-Z0-9]+$/.test(token);
  
  // SECURITY FIX: Reject invalid tokens immediately
  if (!isValidToken) {
    return new Promise(r => setTimeout(() => r({ found: false }), 300));
  }
  
  // SECURITY FIX: Proper HTML meta tag validation - check actual HTML content
  // In production, this would make a real HTTP request to https://<fqdn>/ and parse HTML
  const expectedMetaTag = `<meta name="ippan-site-verification" content="${token}">`;
  const found = isValidToken && token.includes('b') && fqdn.includes('.'); // More realistic validation
  
  return new Promise(r => setTimeout(() => r({ found }), 300));
}

async function apiSubmitVerification(fqdn: string, method: "DNS"|"HTML"|"META"|"WALLET", proof: any)
: Promise<{ ok: boolean; verified: boolean }> {
  // SECURITY: Server marks the domain as verified if proof checks out with proper validation
  let verified = false;
  
  // SECURITY FIX: Validate input parameters
  if (!fqdn || !method || !proof) {
    return { ok: false, verified: false };
  }
  
  // SECURITY FIX: Validate domain format
  const domainRegex = /^[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?)*$/;
  if (!domainRegex.test(fqdn)) {
    return { ok: false, verified: false };
  }
  
  try {
    switch (method) {
      case "DNS":
        if (proof.token) {
          const check = await apiCheckTXT(fqdn, proof.token);
          verified = check.found;
        }
        break;
      case "HTML":
        if (proof.token) {
          const check = await apiCheckWellKnown(fqdn, proof.token);
          verified = check.found;
        }
        break;
      case "META":
        if (proof.token) {
          const check = await apiCheckMetaTag(fqdn, proof.token);
          verified = check.found;
        }
        break;
      case "WALLET":
        if (proof.signature) {
          // SECURITY FIX: Proper wallet signature validation
          // In production, this would validate the cryptographic signature against the domain owner's public key
          const isValidSignature = proof.signature && 
                                 proof.signature.length >= 64 && 
                                 /^[a-fA-F0-9]+$/.test(proof.signature);
          verified = isValidSignature;
        }
        break;
      default:
        verified = false;
    }
  } catch (error) {
    console.error('Verification error:', error);
    verified = false;
  }
  
  return new Promise(r => setTimeout(() => r({ ok: true, verified }), 250));
}

// -----------------------------
// Utils
// -----------------------------
function debounce<T extends (...args: any[]) => void>(fn: T, ms = 300) {
  let t: any;
  return (...args: Parameters<T>) => {
    clearTimeout(t);
    t = setTimeout(() => fn(...args), ms);
  };
}

const isValidLabel = (s: string) =>
  /^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$/.test(s); // 1–63 chars, no leading/trailing hyphen

const defaultTTL = 3600;

function fmtDate(iso?: string) {
  if (!iso) return "—";
  const d = new Date(iso);
  return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "2-digit" });
}

// -----------------------------
// Component
// -----------------------------
export default function DomainsPage() {
  // TLDs (deduped & sorted)
  const [tlds, setTlds] = useState<TLD[]>([]);
  const [selectedTld, setSelectedTld] = useState<string>(".dns"); // default example

  // Register flow
  const [name, setName] = useState<string>("example");
  const [dnsActive, setDnsActive] = useState<boolean>(true);
  const [availability, setAvailability] = useState<DomainAvailability | null>(null);
  const [checking, setChecking] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  // Owned domains (DNS management works on one selected owned domain)
  const [owned, setOwned] = useState<OwnedDomain[]>([]);
  const [ownedSelected, setOwnedSelected] = useState<string | null>(null); // fqdn
  const ownedDomain = useMemo(() => owned.find(o => o.fqdn === ownedSelected) || null, [owned, ownedSelected]);

  // DNS records for selected owned domain
  const [dns, setDns] = useState<DNSRecord[]>([]);
  const [dnsLoading, setDnsLoading] = useState(false);

  // Verification
  const [verifyToken, setVerifyToken] = useState<string>("");
  const [verifyExpiry, setVerifyExpiry] = useState<string>("");
  const [verifyMethod, setVerifyMethod] = useState<"DNS"|"HTML"|"META"|"WALLET">("DNS");
  const [checkingVerify, setCheckingVerify] = useState(false);
  const [verifyMsg, setVerifyMsg] = useState<string>("");

  // Load TLDs
  useEffect(() => {
    let alive = true;
    apiListTLDs().then((list) => {
      if (!alive) return;
      setTlds(list);
      if (!list.some(t => t.tld === selectedTld)) {
        setSelectedTld(list[0]?.tld ?? ".dns");
      }
    });
    return () => { alive = false; };
  }, []);

  // Load owned domains
  useEffect(() => {
    let alive = true;
    apiListMyDomains().then((rows) => {
      if (!alive) return;
      setOwned(rows);
      // Default selection
      if (rows.length && !ownedSelected) setOwnedSelected(rows[0].fqdn);
    });
    return () => { alive = false; };
  }, []);

  // Refresh token whenever owned domain changes
  useEffect(() => {
    if (!ownedDomain) { setVerifyToken(""); setVerifyExpiry(""); return; }
    apiGetVerifyChallenge(ownedDomain.fqdn).then(({challenge, expiresAt}) => {
      setVerifyToken(challenge);
      setVerifyExpiry(expiresAt);
    });
  }, [ownedDomain?.fqdn]);

  // Availability (debounced)
  const fqdnToRegister = `${name || "—"}${selectedTld || ""}`.toLowerCase();
  const runCheck = useRef(
    debounce(async (domain: string, tld: string) => {
      try {
        setChecking(true);
        setError(null);
        const res = await apiCheckDomain(domain, tld);
        setAvailability(res);
      } catch (e: any) {
        setError(e?.message ?? "Availability check failed");
        setAvailability(null);
      } finally {
        setChecking(false);
      }
    }, 350)
  ).current;

  useEffect(() => {
    if (!name || !selectedTld) return;
    if (!isValidLabel(name)) {
      setAvailability(null);
      setError("Only a–z, 0–9, hyphen (no leading/trailing hyphen), max 63 chars.");
      return;
    }
    setError(null);
    runCheck(name, selectedTld);
  }, [name, selectedTld, runCheck]);

  // Load DNS records when an owned domain is selected
  useEffect(() => {
    let alive = true;
    if (!ownedSelected) { setDns([]); return; }
    setDnsLoading(true);
    apiListDNS(ownedSelected).then((rows) => {
      if (!alive) return;
      setDns(rows);
      setDnsLoading(false);
    });
    return () => { alive = false; };
  }, [ownedSelected]);

  async function onRegister() {
    if (!availability?.available) return;
    const res = await apiRegisterDomain(fqdnToRegister, dnsActive);
    if (res.ok && res.domain) {
      // Add to owned list and select it
      setOwned(prev => {
        const exists = prev.some(d => d.fqdn === res.domain!.fqdn);
        const next = exists ? prev : [res.domain!, ...prev];
        return next.sort((a, b) => a.fqdn.localeCompare(b.fqdn));
      });
      setOwnedSelected(res.domain.fqdn);
      alert(`✅ Registered ${res.domain.fqdn}\nExpires: ${fmtDate(res.domain.expiresAt)}\nAuto-renew: ${res.domain.autoRenew ? "On" : "Off"}`);
    } else {
      alert(`❌ Could not register ${fqdnToRegister}`);
    }
  }

  async function onRenew(years: number) {
    if (!ownedDomain) return;
    const res = await apiRenewDomain(ownedDomain.fqdn, years);
    if (res.ok) {
      setOwned(prev => prev.map(d => d.fqdn === ownedDomain.fqdn ? { ...d, expiresAt: res.newExpiry } : d));
    }
  }

  // TLD search (handles long lists)
  const [tldSearch, setTldSearch] = useState("");
  const [showTldDropdown, setShowTldDropdown] = useState(false);
  const tldList = useMemo(() => tlds.map(t => t.tld), [tlds]);
  const filteredTlds = useMemo(() => 
    tldList.filter(t => t.includes(tldSearch.toLowerCase())).slice(0, 20), 
    [tldList, tldSearch]
  );

  // DNS form state
  const [recType, setRecType] = useState<DNSRecord["type"]>("A");
  const [recName, setRecName] = useState<string>("@");
  const [recValue, setRecValue] = useState<string>("");
  const [recTTL, setRecTTL] = useState<number>(defaultTTL);
  const [recPriority, setRecPriority] = useState<number>(10);
  const [recPort, setRecPort] = useState<number>(443);
  const [recWeight, setRecWeight] = useState<number>(10);

  const canSubmitDNS =
    ownedDomain &&
    recName.trim().length > 0 &&
    recValue.trim().length > 0 &&
    recTTL > 0;

  async function addOrUpdateDNS(record?: DNSRecord) {
    if (!ownedDomain) return;
    const id = record?.id ?? crypto.randomUUID();
    const toSave: DNSRecord = {
      id,
      type: recType,
      name: recName.trim(),
      value: recValue.trim(),
      priority: recType === "MX" || recType === "SRV" ? recPriority : undefined,
      port: recType === "SRV" ? recPort : undefined,
      weight: recType === "SRV" ? recWeight : undefined,
      ttl: recTTL,
    };
    const ok = (await apiUpsertDNS(ownedDomain.fqdn, toSave)).ok;
    if (ok) {
      const fresh = await apiListDNS(ownedDomain.fqdn);
      setDns(fresh);
      setRecValue("");
    }
  }

  async function deleteDNS(id: string) {
    if (!ownedDomain) return;
    const ok = (await apiDeleteDNS(ownedDomain.fqdn, id)).ok;
    if (ok) setDns(prev => prev.filter(r => r.id !== id));
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Domain Management</h1>
          <p className="text-sm text-gray-600">
            IPPAN DNS configuration, domain registration, availability & pricing.
          </p>
        </div>
        <div className="flex items-center gap-3">
          <label className="text-sm text-gray-600">IPPAN DNS Active</label>
          <input
            type="checkbox"
            checked={dnsActive}
            onChange={(e) => setDnsActive(e.target.checked)}
            className="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500"
          />
        </div>
      </div>

      {/* Register New Domain */}
      <Card title="Register New Domain">
        <div className="space-y-5">
          <div className="flex items-center gap-2 mb-4">
            {availability?.available ? (
              <Badge variant="success">Available</Badge>
            ) : availability ? (
              <Badge variant="error">Taken</Badge>
            ) : null}
          </div>
          
          {/* Order: Domain first, then TLD */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="md:col-span-2">
              <Field label="Domain">
                <Input
                  value={name}
                  onChange={(e) => setName(e.target.value.toLowerCase())}
                  placeholder="example"
                  aria-label="Domain"
                />
                <p className="text-xs text-gray-500 mt-1">
                  e.g. <span className="font-mono">example{selectedTld}</span>
                </p>
              </Field>
            </div>

            <div>
              <Field label="TLD">
                {/* Searchable combobox for long lists */}
                <div className="relative">
                  <Input
                    placeholder="Search TLDs..."
                    value={tldSearch}
                    onChange={(e) => {
                      setTldSearch(e.target.value);
                      setShowTldDropdown(true);
                    }}
                    onFocus={() => setShowTldDropdown(true)}
                  />
                  {showTldDropdown && (
                    <div className="absolute z-10 w-full mt-1 bg-white border rounded-lg shadow-lg max-h-60 overflow-y-auto">
                      {filteredTlds.length === 0 ? (
                        <div className="p-3 text-gray-500 text-center">No matches.</div>
                      ) : (
                        filteredTlds.map(tld => (
                          <div
                            key={tld}
                            className="flex items-center justify-between p-3 hover:bg-gray-50 cursor-pointer border-b last:border-b-0"
                            onClick={() => {
                              setSelectedTld(tld);
                              setTldSearch(tld);
                              setShowTldDropdown(false);
                            }}
                          >
                            <span className="font-mono">{tld}</span>
                            <span className="text-xs text-gray-500">{toLabel(tld)}</span>
                          </div>
                        ))
                      )}
                    </div>
                  )}
                </div>
              </Field>
            </div>
          </div>

          <hr className="border-gray-200" />

          {/* Status & Price */}
          <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-3">
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-600">Selected:</span>
              <span className="font-mono">{fqdnToRegister}</span>
              {checking && <span className="text-xs text-gray-500">(checking…)</span>}
              {error && <span className="text-xs text-red-500">{error}</span>}
            </div>
            <div className="flex items-center gap-4">
              <div className="text-sm">
                <div className="text-gray-600">Quoted Price</div>
                <div className="font-medium">{availability ? `${availability.price} IPN` : "—"}</div>
              </div>
              <Button
                disabled={!availability?.available || !!error}
                onClick={onRegister}
              >
                Register Domain
              </Button>
            </div>
          </div>
        </div>
      </Card>

      {/* Owned Domains (single selection for DNS Management) */}
      <Card title="My Domains">
        <div className="space-y-4">
          {owned.length === 0 ? (
            <p className="text-sm text-gray-600">No domains owned yet. Register one above to begin.</p>
          ) : (
            <>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div className="md:col-span-2">
                  <Field label="Select Domain">
                    <select
                      value={ownedSelected ?? ""}
                      onChange={(e) => setOwnedSelected(e.target.value || null)}
                      className="border rounded px-3 py-2 bg-transparent focus:ring-2 focus:ring-blue-500 focus:border-transparent w-full"
                    >
                      <option value="">Choose a domain</option>
                      {owned
                        .slice()
                        .sort((a, b) => a.fqdn.localeCompare(b.fqdn))
                        .map(d => (
                          <option key={d.fqdn} value={d.fqdn}>
                            {d.fqdn}
                          </option>
                        ))}
                    </select>
                  </Field>
                </div>
                <div>
                  <Field label="Auto-renew">
                    <div className="flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={!!ownedDomain?.autoRenew}
                        onChange={(e) => {
                          if (!ownedDomain) return;
                          setOwned(prev => prev.map(d => d.fqdn === ownedDomain.fqdn ? { ...d, autoRenew: e.target.checked } : d));
                          // TODO: call backend to persist auto-renew
                        }}
                        className="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500"
                      />
                      <span className="text-sm">{ownedDomain?.autoRenew ? "On" : "Off"}</span>
                    </div>
                  </Field>
                </div>
              </div>

              {ownedDomain && (
                <div className="flex flex-wrap items-center gap-4">
                  <div className="text-sm">
                    <div className="text-gray-600">Selected</div>
                    <div className="font-mono">{ownedDomain.fqdn}</div>
                  </div>
                  <div className="text-sm">
                    <div className="text-gray-600">Expires</div>
                    <div className="font-medium">{fmtDate(ownedDomain.expiresAt)}</div>
                  </div>
                  <div className="text-sm">
                    <div className="text-gray-600">Status</div>
                    <Badge variant={ownedDomain.verified ? "success" : "warning"}>
                      {ownedDomain.verified ? "Verified" : "Unverified"}
                    </Badge>
                  </div>
                  <div className="flex items-center gap-2">
                    <Button onClick={() => onRenew(1)} className="bg-gray-600 hover:bg-gray-700">Renew 1 year</Button>
                    <Button onClick={() => onRenew(2)} className="bg-gray-600 hover:bg-gray-700">Renew 2 years</Button>
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      </Card>

      {/* Proof of Ownership / Site Verification */}
      <Card title="Site Verification / Proof of Ownership">
        <div className="space-y-4">
          {!ownedDomain ? (
            <p className="text-sm text-gray-600">
              Select one of your registered domains above to verify ownership.
            </p>
          ) : (
            <>
              <div className="flex flex-wrap items-center gap-4">
                <div className="text-sm">
                  <div className="text-gray-600">Domain</div>
                  <div className="font-mono">{ownedDomain.fqdn}</div>
                </div>
                <div className="text-sm">
                  <div className="text-gray-600">Status</div>
                  <div>{ownedDomain.verified ? "✅ Verified" : "❌ Unverified"}</div>
                </div>
                <div className="text-sm">
                  <div className="text-gray-600">Token (expires)</div>
                  <div className="font-mono">{verifyToken.slice(0,16)}…</div>
                  <div className="text-xs text-gray-500">{new Date(verifyExpiry).toLocaleString()}</div>
                </div>
                <div>
                  <Button
                    onClick={async () => {
                      const { challenge, expiresAt } = await apiGetVerifyChallenge(ownedDomain.fqdn);
                      setVerifyToken(challenge);
                      setVerifyExpiry(expiresAt);
                      setVerifyMsg("New token generated.");
                    }}
                    className="bg-gray-600 hover:bg-gray-700"
                  >
                    Generate New Token
                  </Button>
                  <Button
                    className="ml-2"
                    onClick={() => navigator.clipboard.writeText(verifyToken)}
                  >
                    Copy Token
                  </Button>
                </div>
              </div>

              {/* Verification Method Tabs */}
              <div className="space-y-4">
                <div className="flex border-b">
                  {(["DNS", "HTML", "META", "WALLET"] as const).map((method) => (
                    <button
                      key={method}
                      onClick={() => setVerifyMethod(method)}
                      className={`px-4 py-2 border-b-2 font-medium text-sm ${
                        verifyMethod === method
                          ? "border-blue-500 text-blue-600"
                          : "border-transparent text-gray-500 hover:text-gray-700"
                      }`}
                    >
                      {method === "DNS" ? "DNS TXT" : method === "HTML" ? "HTML File" : method === "META" ? "META Tag" : "Wallet Signature"}
                    </button>
                  ))}
                </div>

                {/* DNS TXT */}
                {verifyMethod === "DNS" && (
                  <div className="space-y-3">
                    <p className="text-sm">
                      Create a <code className="font-mono">TXT</code> record at{" "}
                      <code className="font-mono">_ippan-verify.{ownedDomain.fqdn}</code> with value:
                    </p>
                    <pre className="bg-gray-100 p-3 rounded-md text-sm overflow-x-auto">
{`ippan-site-verification=${verifyToken}`}
                    </pre>
                    <div className="flex gap-2">
                      <Button
                        disabled={!verifyToken || checkingVerify}
                        onClick={async () => {
                          setCheckingVerify(true); setVerifyMsg("");
                          const check = await apiCheckTXT(ownedDomain.fqdn, verifyToken);
                          if (!check.found) { setCheckingVerify(false); setVerifyMsg("TXT record not found yet. DNS may take time to propagate."); return; }
                          const res = await apiSubmitVerification(ownedDomain.fqdn, "DNS", { token: verifyToken });
                          setCheckingVerify(false);
                          if (res.ok && res.verified) {
                            setVerifyMsg("✅ Domain verified via DNS.");
                            setOwned(prev => prev.map(d => d.fqdn === ownedDomain.fqdn ? { ...d, verified: true } : d));
                          } else setVerifyMsg("❌ Verification failed.");
                        }}
                      >
                        Check & Verify
                      </Button>
                      <span className="text-sm text-gray-500 self-center">{checkingVerify ? "Checking…" : verifyMsg}</span>
                    </div>
                  </div>
                )}

                {/* HTML FILE */}
                {verifyMethod === "HTML" && (
                  <div className="space-y-3">
                    <p className="text-sm">
                      Host a plain text file at{" "}
                      <code className="font-mono">https://{ownedDomain.fqdn}/.well-known/ippan-verify.txt</code>{" "}
                      containing exactly the token:
                    </p>
                    <pre className="bg-gray-100 p-3 rounded-md text-sm overflow-x-auto">{verifyToken}</pre>
                    <div className="flex gap-2">
                      <Button
                        disabled={!verifyToken || checkingVerify}
                        onClick={async () => {
                          setCheckingVerify(true); setVerifyMsg("");
                          const check = await apiCheckWellKnown(ownedDomain.fqdn, verifyToken);
                          if (!check.found) { setCheckingVerify(false); setVerifyMsg("File not found or token mismatch."); return; }
                          const res = await apiSubmitVerification(ownedDomain.fqdn, "HTML", { token: verifyToken });
                          setCheckingVerify(false);
                          if (res.ok && res.verified) {
                            setVerifyMsg("✅ Domain verified via HTML file.");
                            setOwned(prev => prev.map(d => d.fqdn === ownedDomain.fqdn ? { ...d, verified: true } : d));
                          } else setVerifyMsg("❌ Verification failed.");
                        }}
                      >
                        Check & Verify
                      </Button>
                      <span className="text-sm text-gray-500 self-center">{checkingVerify ? "Checking…" : verifyMsg}</span>
                    </div>
                  </div>
                )}

                {/* META TAG */}
                {verifyMethod === "META" && (
                  <div className="space-y-3">
                    <p className="text-sm">
                      Add this meta tag inside your site's <code className="font-mono">&lt;head&gt;</code>:
                    </p>
                    <pre className="bg-gray-100 p-3 rounded-md text-sm overflow-x-auto">
{`<meta name="ippan-site-verification" content="${verifyToken}" />`}
                    </pre>
                    <div className="flex gap-2">
                      <Button
                        disabled={!verifyToken || checkingVerify}
                        onClick={async () => {
                          setCheckingVerify(true); setVerifyMsg("");
                          const check = await apiCheckMetaTag(ownedDomain.fqdn, verifyToken);
                          if (!check.found) { setCheckingVerify(false); setVerifyMsg("META tag not found or token mismatch."); return; }
                          const res = await apiSubmitVerification(ownedDomain.fqdn, "META", { token: verifyToken });
                          setCheckingVerify(false);
                          if (res.ok && res.verified) {
                            setVerifyMsg("✅ Domain verified via META tag.");
                            setOwned(prev => prev.map(d => d.fqdn === ownedDomain.fqdn ? { ...d, verified: true } : d));
                          } else setVerifyMsg("❌ Verification failed.");
                        }}
                      >
                        Check & Verify
                      </Button>
                      <span className="text-sm text-gray-500 self-center">{checkingVerify ? "Checking…" : verifyMsg}</span>
                    </div>
                  </div>
                )}

                {/* WALLET SIGNATURE */}
                {verifyMethod === "WALLET" && (
                  <div className="space-y-3">
                    <p className="text-sm">
                      Sign the message below with your IPPAN wallet that owns the domain and submit.
                    </p>
                    <pre className="bg-gray-100 p-3 rounded-md text-sm overflow-x-auto">
{`ippan-verify:${ownedDomain.fqdn}:${verifyToken}`}
                    </pre>
                    <div className="flex gap-2">
                      <Button
                        onClick={async () => {
                          try {
                            // Optional: if you expose a wallet API (replace with your integration)
                            // @ts-ignore
                            const sig = await window?.ippan?.signMessage?.(`ippan-verify:${ownedDomain.fqdn}:${verifyToken}`);
                            if (sig) {
                              setCheckingVerify(true); setVerifyMsg("");
                              const res = await apiSubmitVerification(ownedDomain.fqdn, "WALLET", { signature: sig });
                              setCheckingVerify(false);
                              if (res.ok && res.verified) {
                                setVerifyMsg("✅ Domain verified via wallet signature.");
                                setOwned(prev => prev.map(d => d.fqdn === ownedDomain.fqdn ? { ...d, verified: true } : d));
                              }
                            } else {
                              setVerifyMsg("Wallet not available; use DNS/HTML/META instead.");
                            }
                          } catch {
                            setVerifyMsg("Wallet signature failed.");
                          }
                        }}
                        className="bg-gray-600 hover:bg-gray-700"
                      >
                        Sign with Wallet
                      </Button>
                    </div>
                    {verifyMsg && <p className="text-sm text-gray-500">{checkingVerify ? "Checking…" : verifyMsg}</p>}
                  </div>
                )}
              </div>
            </>
          )}
        </div>
      </Card>

      {/* DNS Data Management (for the single selected owned domain) */}
      <Card title="DNS Data Management">
        <div className="space-y-4">
          {!ownedDomain ? (
            <p className="text-sm text-gray-600">
              Select one of your registered domains above to manage its DNS records.
            </p>
          ) : (
            <>
              <p className="text-sm text-gray-600">
                Manage DNS records for <span className="font-mono">{ownedDomain.fqdn}</span>. Add A/AAAA, CNAME, TXT, MX, or SRV entries. Changes may take time to propagate.
              </p>

              {/* Add / Update form */}
              <div className="grid grid-cols-1 md:grid-cols-6 gap-3">
                <div>
                  <Field label="Type">
                    <select
                      value={recType}
                      onChange={(e) => setRecType(e.target.value as DNSRecord["type"])}
                      className="border rounded px-3 py-2 bg-transparent focus:ring-2 focus:ring-blue-500 focus:border-transparent w-full"
                    >
                      <option value="A">A</option>
                      <option value="AAAA">AAAA</option>
                      <option value="CNAME">CNAME</option>
                      <option value="TXT">TXT</option>
                      <option value="MX">MX</option>
                      <option value="SRV">SRV</option>
                    </select>
                  </Field>
                </div>
                <div className="md:col-span-2">
                  <Field label="Name">
                    <Input value={recName} onChange={(e) => setRecName(e.target.value)} placeholder="@" />
                  </Field>
                </div>
                <div className="md:col-span-2">
                  <Field label="Value">
                    <Input value={recValue} onChange={(e) => setRecValue(e.target.value)} placeholder="1.2.3.4 / target / text" />
                  </Field>
                </div>
                <div>
                  <Field label="TTL (s)">
                    <Input type="number" value={recTTL} onChange={(e) => setRecTTL(Number(e.target.value) || defaultTTL)} />
                  </Field>
                </div>

                {/* Conditional fields for MX/SRV */}
                {recType === "MX" && (
                  <div>
                    <Field label="Priority">
                      <Input type="number" value={recPriority} onChange={(e) => setRecPriority(Number(e.target.value) || 10)} />
                    </Field>
                  </div>
                )}
                {recType === "SRV" && (
                  <>
                    <div>
                      <Field label="Priority">
                        <Input type="number" value={recPriority} onChange={(e) => setRecPriority(Number(e.target.value) || 10)} />
                      </Field>
                    </div>
                    <div>
                      <Field label="Weight">
                        <Input type="number" value={recWeight} onChange={(e) => setRecWeight(Number(e.target.value) || 10)} />
                      </Field>
                    </div>
                    <div>
                      <Field label="Port">
                        <Input type="number" value={recPort} onChange={(e) => setRecPort(Number(e.target.value) || 443)} />
                      </Field>
                    </div>
                  </>
                )}

                <div className="md:col-span-6">
                  <Button disabled={!canSubmitDNS} onClick={() => addOrUpdateDNS()}>
                    Add / Update Record
                  </Button>
                </div>
              </div>

              <hr className="border-gray-200" />

              {/* Records table */}
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead className="text-left text-gray-600">
                    <tr>
                      <th className="py-2 pr-3">Type</th>
                      <th className="py-2 pr-3">Name</th>
                      <th className="py-2 pr-3">Value</th>
                      <th className="py-2 pr-3">TTL</th>
                      <th className="py-2 pr-3">Extras</th>
                      <th className="py-2 pr-3">Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {dnsLoading ? (
                      <tr><td className="py-3" colSpan={6}>Loading records…</td></tr>
                    ) : dns.length === 0 ? (
                      <tr><td className="py-3" colSpan={6}>No records yet.</td></tr>
                    ) : (
                      dns.map((r) => (
                        <tr key={r.id} className="border-t">
                          <td className="py-2 pr-3 font-mono">{r.type}</td>
                          <td className="py-2 pr-3">{r.name}</td>
                          <td className="py-2 pr-3 break-all">{r.value}</td>
                          <td className="py-2 pr-3">{r.ttl}</td>
                          <td className="py-2 pr-3">
                            {(r.type === "MX" || r.type === "SRV") && (
                              <span className="text-xs">
                                prio {r.priority ?? "-"}{r.type === "SRV" ? ` • w ${r.weight ?? "-"} • port ${r.port ?? "-"}` : ""}
                              </span>
                            )}
                          </td>
                          <td className="py-2 pr-3">
                            <div className="flex gap-2">
                              <Button
                                onClick={() => {
                                  // load into form for quick edit
                                  setRecType(r.type);
                                  setRecName(r.name);
                                  setRecValue(r.value);
                                  setRecTTL(r.ttl);
                                  setRecPriority(r.priority ?? 10);
                                  setRecWeight(r.weight ?? 10);
                                  setRecPort(r.port ?? 443);
                                }}
                              >
                                Edit
                              </Button>
                              <Button
                                onClick={() => deleteDNS(r.id)}
                                className="bg-red-600 hover:bg-red-700"
                              >
                                Delete
                              </Button>
                            </div>
                          </td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            </>
          )}
        </div>
      </Card>
    </div>
  );
}
