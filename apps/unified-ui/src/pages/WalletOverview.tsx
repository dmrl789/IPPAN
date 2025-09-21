import { useState, useEffect, useMemo } from 'react'
import QRCode from 'qrcode'
import { Card, Button, Field, Input, Badge } from '../components/UI'
import { getNodeStatus, getNetworkStats, getMempoolStats, getConsensusStats, getHealth } from '../lib/api'

// ---------------------------------
// Types
// ---------------------------------
type Asset = { 
  symbol: string; 
  name: string; 
  balance: number; 
};

type Activity = { 
  id: string; 
  title: string; 
  subtitle: string; 
  delta: number; 
  when: string; 
};

type Staking = { 
  staked: number; 
  rewards: number; 
};

type WalletType = "watch-only" | "local";

type WalletState = {
  address: string;
  assets: Asset[];
  staking: Staking;
  domains: string[];
  activities: Activity[];
};

type AddressBookEntry = { 
  name: string; 
  address: string; 
};

type FeePreview = { 
  fee: number; 
  maxFee: number; 
  nonce: number; 
  etaSeconds: number; 
};

type Fx = { 
  USD: number; 
  EUR: number; 
};

// ---------------------------------
// Mock / placeholder APIs (replace with real ones)
// ---------------------------------
function randHex(len = 39) {
  const bytes = new Uint8Array(Math.ceil(len / 2));
  crypto.getRandomValues(bytes);
  return Array.from(bytes, b => b.toString(16).padStart(2, "0")).join("").slice(0, len);
}

// Generate random mixed-case alphanumeric string
const randMixed = (len: number): string => {
  console.log('randMixed called with len:', len);
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  let result = '';
  
  // Use crypto.getRandomValues for better randomness
  const randomBytes = new Uint8Array(len);
  crypto.getRandomValues(randomBytes);
  
  for (let i = 0; i < len; i++) {
    // Use the random byte to select a character
    const randomIndex = randomBytes[i] % chars.length;
    result += chars.charAt(randomIndex);
  }
  
  // Debug logging
  console.log('randMixed result:', {
    result,
    length: result.length,
    uniqueChars: new Set(result.split('')).size,
    expectedLength: len
  });
  
  return result;
};


function nowMinus(hours: number) {
  const d = new Date(Date.now() - hours * 3600 * 1000);
  return d.toLocaleString();
}

async function apiGetWallet(address: string): Promise<WalletState> {
  try {
    // Import the real API functions
    const { getWalletBalance, getWalletTransactions } = await import('../lib/walletApi');
    
    // Get real wallet balance from blockchain
    const balanceData = await getWalletBalance(address);
    
    // Get real transactions
    const transactions = await getWalletTransactions(address);
    
    // Convert to UI format
    const main: Asset = { 
      symbol: "IPN", 
      name: "IPPAN", 
      balance: balanceData.balance / 1000000000 // Convert from smallest units
    };
    
    const tokens: Asset[] = [
      main,
      { 
        symbol: "STAKE", 
        name: "Staked IPN (sIPN)", 
        balance: balanceData.staked / 1000000000 // Convert from smallest units
      },
    ];
    
    // Convert transactions to activities
    const acts: Activity[] = transactions.slice(0, 10).map(tx => ({
      id: tx.id || crypto.randomUUID(),
      title: tx.type === 'send' ? "Sent Payment" : tx.type === 'receive' ? "Received Payment" : "Transaction",
      subtitle: tx.type === 'send' ? `To: ${tx.to}` : tx.type === 'receive' ? `From: ${tx.from}` : tx.description || "Blockchain transaction",
      delta: tx.type === 'send' ? -tx.amount : tx.type === 'receive' ? +tx.amount : 0,
      when: new Date(tx.timestamp).toLocaleString()
    }));
    
    const staking: Staking = { 
      staked: balanceData.staked / 1000000000, 
      rewards: 0 // TODO: Calculate actual rewards
    };
    
    const domains: string[] = []; // TODO: Get real domains from blockchain
    
    return { address, assets: tokens, activities: acts, staking, domains };
  } catch (error) {
    console.error('Error fetching wallet data:', error);
    // Fallback to empty state
    const main: Asset = { symbol: "IPN", name: "IPPAN", balance: 0 };
    return { 
      address, 
      assets: [main], 
      activities: [], 
      staking: { staked: 0, rewards: 0 }, 
      domains: [] 
    };
  }
}

async function apiEstimateFee(_from: string, _to: string, amount: number, priority: number): Promise<FeePreview> {
  const base = Math.max(0.01, Math.min(1, amount * 0.002));
  const mult = 0.5 + priority * 0.5; // priority in [0..1] => 0.5x..1x
  const fee = Number((base * mult).toFixed(4));
  const maxFee = Number((fee * (1.25 + priority * 0.5)).toFixed(4));
  const nonce = Math.floor(Math.random() * 1000);
  const etaSeconds = Math.max(2, Math.round(6 - priority * 4));
  return new Promise((r) => setTimeout(() => r({ fee, maxFee, nonce, etaSeconds }), 220));
}

async function apiSendPayment(from: string, to: string, amount: number, fee: number, nonce: number, memo?: string) {
  console.log("SEND", { from, to, amount, fee, nonce, memo });
  return new Promise<{ ok: boolean }>((r) => setTimeout(() => r({ ok: true }), 450));
}

// Check if address exists in explorer
async function apiExplorerCheckAddressExists(addr: string): Promise<boolean> {
  // Validate address format: i + 38 alphanumeric characters
  const addressRegex = /^i[A-Za-z0-9]{38}$/;
  if (!addressRegex.test(addr)) {
    throw new Error('Invalid address format. Must be i followed by 38 alphanumeric characters.');
  }
  // simulate explorer lookup (80% chance it exists)
  return new Promise((r) => setTimeout(() => r(Math.random() > 0.2 && /^i[A-Za-z0-9]{38}$/.test(addr)), 180));
}

async function apiRates(): Promise<Fx> {
  // Mock FX: 1 IPN = 0.85 USD, 0.78 EUR (replace with your oracle)
  return new Promise((r) => setTimeout(() => r({ USD: 0.85, EUR: 0.78 }), 150));
}

// Seed / import (mock)
const WORDS = ["apple","robot","silver","delta","piano","garden","matrix","orbit","pixel","quantum","river","solstice","tiger","ultra","vector","willow","xenon","yodel","zebra","amber","breeze","crystal","dynamo","ember"];
function generateSeed(n = 12) {
  const arr: string[] = [];
  for (let i = 0; i < n; i++) arr.push(WORDS[Math.floor(Math.random() * WORDS.length)]);
  return arr.join(" ");
}
// Generate address from seed using proper cryptographic hashing
const addressFromSeed = async (seed: string): Promise<string> => {
  try {
    // Use Web Crypto API for proper SHA-256 hashing
    const encoder = new TextEncoder();
    const data = encoder.encode(seed);
    const hashBuffer = await crypto.subtle.digest('SHA-256', data);
    
    // Convert hash to Uint8Array
    const hashArray = new Uint8Array(hashBuffer);
    
    // Use the hash to generate a deterministic but varied address
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let result = '';
    
    // Generate exactly 38 characters using the hash data
    for (let i = 0; i < 38; i++) {
      // Use different bytes from the hash for each position
      const hashIndex = i % hashArray.length;
      const hashByte = hashArray[hashIndex];
      
      // Mix with position to ensure variety
      const mixedValue = (hashByte + i * 7) % chars.length;
      result += chars.charAt(mixedValue);
    }
    
    return "i" + result; // i + 38 alphanumeric = 39 total
  } catch (error) {
    // Fallback to a simpler method if crypto.subtle is not available
    console.warn('Web Crypto API not available, using fallback method');
    
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let result = '';
    let hash = 0;
    
    // Simple hash function
    for (let i = 0; i < seed.length; i++) {
      hash = ((hash << 5) - hash + seed.charCodeAt(i)) & 0xFFFFFFFF;
    }
    
    // Generate exactly 38 characters
    for (let i = 0; i < 38; i++) {
      const mixedHash = (hash + i * 31) & 0xFFFFFFFF;
      const charIndex = mixedHash % chars.length;
      result += chars.charAt(charIndex);
      hash = (hash * 33 + mixedHash) & 0xFFFFFFFF;
    }
    
    return "i" + result;
  }
};
// Generate address from private key using proper cryptographic hashing
const addressFromPrivKey = async (pk: string): Promise<string> => {
  try {
    // Use Web Crypto API for proper SHA-256 hashing
    const encoder = new TextEncoder();
    const data = encoder.encode(pk);
    const hashBuffer = await crypto.subtle.digest('SHA-256', data);
    
    // Convert hash to Uint8Array
    const hashArray = new Uint8Array(hashBuffer);
    
    // Use the hash to generate a deterministic but varied address
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let result = '';
    
    // Generate exactly 38 characters using the hash data
    for (let i = 0; i < 38; i++) {
      // Use different bytes from the hash for each position
      const hashIndex = i % hashArray.length;
      const hashByte = hashArray[hashIndex];
      
      // Mix with position to ensure variety
      const mixedValue = (hashByte + i * 11) % chars.length;
      result += chars.charAt(mixedValue);
    }
    
    return "i" + result; // i + 38 alphanumeric = 39 total
  } catch (error) {
    // Fallback to a simpler method if crypto.subtle is not available
    console.warn('Web Crypto API not available, using fallback method');
    
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let result = '';
    let hash = 0;
    
    // Simple hash function
    for (let i = 0; i < pk.length; i++) {
      hash = ((hash << 5) - hash + pk.charCodeAt(i)) & 0xFFFFFFFF;
    }
    
    // Generate exactly 38 characters
    for (let i = 0; i < 38; i++) {
      const mixedHash = (hash + i * 37) & 0xFFFFFFFF;
      const charIndex = mixedHash % chars.length;
      result += chars.charAt(charIndex);
      hash = (hash * 37 + mixedHash) & 0xFFFFFFFF;
    }
    
    return "i" + result;
  }
};

// ---------------------------------
// Local storage helpers
// ---------------------------------
const LS_ADDR = "ippan.wallet.address";
const LS_TYPE = "ippan.wallet.type";
const LS_BOOK = "ippan.wallet.addressbook";
const LS_LIMIT = "ippan.wallet.dailyLimit";

function saveAddress(addr: string | null) { 
  if (addr) localStorage.setItem(LS_ADDR, addr); 
  else localStorage.removeItem(LS_ADDR); 
}

function loadAddress(): string | null { 
  return localStorage.getItem(LS_ADDR); 
}

function saveType(t: WalletType) { 
  localStorage.setItem(LS_TYPE, t); 
}

function loadType(): WalletType { 
  return (localStorage.getItem(LS_TYPE) as WalletType) || "watch-only"; 
}

function saveBook(book: AddressBookEntry[]) { 
  console.log('saveBook called:', book);
  localStorage.setItem(LS_BOOK, JSON.stringify(book)); 
  console.log('Saved to localStorage');
}

function loadBook(): AddressBookEntry[] { 
  try { 
    return JSON.parse(localStorage.getItem(LS_BOOK) || "[]"); 
  } catch { 
    return []; 
  } 
}

function saveLimit(v: number) { 
  localStorage.setItem(LS_LIMIT, String(v)); 
}

function loadLimit(): number { 
  return Number(localStorage.getItem(LS_LIMIT) || "0"); 
}

function resetSession() {
  [LS_ADDR, LS_BOOK, LS_TYPE].forEach(k => localStorage.removeItem(k));
}

// ---------------------------------
// Utilities
// ---------------------------------
function copy(text: string) { 
  navigator.clipboard?.writeText(text); 
}

function downloadCSV(filename: string, rows: string[][]) {
  const csv = rows.map(r => r.map(c => `"${String(c).replace(/"/g,'""')}"`).join(",")).join("\n");
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url; a.download = filename; a.click();
  URL.revokeObjectURL(url);
}

function fmtMoney(n: number, cur = "USD") { 
  return new Intl.NumberFormat(undefined, { style: "currency", currency: cur }).format(n); 
}

// ---------------------------------
// Component
// ---------------------------------
export default function WalletOverview() {
  const [address, setAddress] = useState<string | null>(loadAddress());
  const [walletType, setWalletType] = useState<WalletType>(loadType());
  const [inputAddr, setInputAddr] = useState("");
  const [state, setState] = useState<WalletState | null>(null);
  const [loading, setLoading] = useState(false);
  const [addrBook, setAddrBook] = useState<AddressBookEntry[]>(loadBook());
  const [fx, setFx] = useState<Fx>({ USD: 0.85, EUR: 0.78 });

  // Real-time node data
  const [nodeData, setNodeData] = useState({
    status: null,
    network: null,
    mempool: null,
    consensus: null,
    health: null,
    loading: true,
    error: null
  });

  // Fetch real-time node data
  useEffect(() => {
    const fetchNodeData = async () => {
      try {
        setNodeData(prev => ({ ...prev, loading: true, error: null }));
        
        const [health, status, network, mempool, consensus] = await Promise.allSettled([
          getHealth(),
          getNodeStatus(),
          getNetworkStats(),
          getMempoolStats(),
          getConsensusStats()
        ]);

        setNodeData({
          health: health.status === 'fulfilled' ? health.value : null,
          status: status.status === 'fulfilled' ? status.value : null,
          network: network.status === 'fulfilled' ? network.value : null,
          mempool: mempool.status === 'fulfilled' ? mempool.value : null,
          consensus: consensus.status === 'fulfilled' ? consensus.value : null,
          loading: false,
          error: null
        });
      } catch (error) {
        setNodeData(prev => ({ 
          ...prev, 
          loading: false, 
          error: error.message || 'Failed to fetch node data' 
        }));
      }
    };

    fetchNodeData();
    const interval = setInterval(fetchNodeData, 5000); // Update every 5 seconds
    return () => clearInterval(interval);
  }, []);

  // Send drawer
  const [sendOpen, setSendOpen] = useState(false);
  const [sendTo, setSendTo] = useState("");
  const [sendAmount, setSendAmount] = useState("");
  const [sendMemo, setSendMemo] = useState("");
  const [priority, setPriority] = useState<number>(0.5); // 0..1
  const [feePreview, setFeePreview] = useState<FeePreview | null>(null);
  const [nonceOverride, setNonceOverride] = useState<string>("");
  const [checkingRecipient, setCheckingRecipient] = useState<"idle"|"checking"|"ok"|"bad">("idle");

  // Receive modal
  const [recvOpen, setRecvOpen] = useState(false);
  const [qr, setQr] = useState<string>("");

  // Create/Import dialogs
  const [createImportOpen, setCreateImportOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<"create" | "import">("create");
  const [seed, setSeed] = useState(generateSeed(12));
  const [priv, setPriv] = useState("");
  const [revealSeed, setRevealSeed] = useState(false);

  // Address book
  const [newContactName, setNewContactName] = useState("");
  const [newContactAddr, setNewContactAddr] = useState("");
  const [addrBookSearch, setAddrBookSearch] = useState("");
  const [selectedContact, setSelectedContact] = useState<string | null>(null);

  // Security
  const [dailyLimit, setDailyLimit] = useState<number>(loadLimit());
  const [devices, setDevices] = useState<{ id: string; name: string; last: string }[]>([
    { id: crypto.randomUUID(), name: navigator.userAgent.slice(0, 60) + "...", last: new Date().toLocaleString() }
  ]);

  // Load FX and wallet snapshot
  useEffect(() => { 
    apiRates().then(setFx).catch(() => {}); 
  }, []);

  useEffect(() => {
    let alive = true;
    if (!address) { setState(null); return; }
    setLoading(true);
    apiGetWallet(address).then(s => {
      if (!alive) return;
      setState(s); setLoading(false);
    });
    return () => { alive = false; };
  }, [address]);

  // Update fee preview when inputs change
  useEffect(() => {
    const amt = Number(sendAmount);
    if (!address || !/^i[A-Za-z0-9]{38}$/.test(sendTo) || !amt || amt <= 0) { 
      setFeePreview(null); 
      return; 
    }
    let alive = true;
    apiEstimateFee(address, sendTo, amt, priority).then(p => { 
      if (alive) setFeePreview(p); 
    });
    return () => { alive = false; };
  }, [address, sendTo, sendAmount, priority]);

  // QR for Receive
  useEffect(() => {
    if (!address || !recvOpen) { setQr(""); return; }
    QRCode.toDataURL(address, { margin: 1, scale: 6 }).then(setQr).catch(() => setQr(""));
  }, [address, recvOpen]);

  const hasWallet = !!address;
  const mainBalance = useMemo(() => state?.assets.find(a => a.symbol === "IPN")?.balance ?? 0, [state?.assets]);
  const totalUSD = useMemo(() => (state?.assets || []).reduce((s,a)=> s + (a.symbol==="IPN" ? a.balance*fx.USD : 0), 0), [state?.assets, fx]);
  const totalEUR = useMemo(() => (state?.assets || []).reduce((s,a)=> s + (a.symbol==="IPN" ? a.balance*fx.EUR : 0), 0), [state?.assets, fx]);
  const filteredContacts = useMemo(() => 
    addrBook.filter(c => 
      c.name.toLowerCase().includes(addrBookSearch.toLowerCase()) ||
      c.address.toLowerCase().includes(addrBookSearch.toLowerCase())
    ), [addrBook, addrBookSearch]
  );

  function setConnected(addr: string, type: WalletType) {
    console.log('setConnected called with:', { addr, type });
    setAddress(addr); 
    saveAddress(addr);
    setWalletType(type); 
    saveType(type);
    
    console.log('Wallet state updated, dispatching event');
    // Dispatch custom event to notify App component of wallet state change
    window.dispatchEvent(new CustomEvent('walletStateChanged'));
    console.log('Event dispatched');
  }


  function onConnectByAddress() {
    const trimmedAddr = inputAddr.trim();
    
    // SECURITY FIX: Proper IPPAN address validation (35 characters: i + 34 alphanumeric)
    if (!/^i[A-Za-z0-9]{34}$/.test(trimmedAddr)) { 
      alert("Enter a valid IPPAN address (i followed by 34 alphanumeric characters)"); 
      return; 
    }
    
    // Additional validation: check for common invalid patterns
    if (trimmedAddr.length !== 35) {
      alert("IPPAN address must be exactly 35 characters long");
      return;
    }
    
    console.log('Connecting wallet with address:', trimmedAddr);
    setConnected(trimmedAddr, "watch-only");
    console.log('Wallet connection completed');
  }


  function onDisconnect() {
    setAddress(null); saveAddress(null);
    setState(null);
    
    // Dispatch custom event to notify App component of wallet state change
    window.dispatchEvent(new CustomEvent('walletStateChanged'));
  }

  function addToBook(name: string, addr: string) {
    console.log('addToBook called:', { name, addr, currentBook: addrBook });
    const next = [...addrBook.filter(e => e.address !== addr), { name, address: addr }];
    console.log('New address book:', next);
    setAddrBook(next); saveBook(next);
  }

  async function onSend() {
    if (!address) return;
    if (walletType === "watch-only") { 
      alert("Watch-only wallet cannot sign transactions. Connect a signer."); 
      return; 
    }
          if (!/^i[A-Za-z0-9]{38}$/.test(sendTo.trim())) { 
      alert("Invalid recipient address"); 
      return; 
    }
    const amt = Number(sendAmount);
    if (!amt || amt <= 0) { 
      alert("Invalid amount"); 
      return; 
    }
    if (dailyLimit > 0 && amt > dailyLimit) {
      if (!confirm(`Amount exceeds your daily limit (${dailyLimit} IPN). Continue?`)) return;
    }
    const fee = feePreview?.fee ?? 0.01;
    const nonce = nonceOverride ? Number(nonceOverride) : (feePreview?.nonce ?? 0);
    const ok = (await apiSendPayment(address, sendTo.trim(), amt, fee, nonce, sendMemo)).ok;
    if (ok) {
      setSendOpen(false);
      setSendTo(""); setSendAmount(""); setSendMemo(""); setNonceOverride("");
      const s = await apiGetWallet(address);
      setState(s);
    } else alert("Payment failed");
  }

  async function checkRecipient() {
    if (!sendTo) return;
    setCheckingRecipient("checking");
    const ok = await apiExplorerCheckAddressExists(sendTo.trim());
    setCheckingRecipient(ok ? "ok" : "bad");
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Wallet Overview</h1>
          <p className="text-sm text-gray-600">
            Connect your wallet to view balances, manage transactions, and access IPPAN features.
          </p>
        </div>
      </div>

      {/* Connect Wallet */}
      <Card title="Connect Wallet">
        <div className="space-y-4">
          <div className="grid gap-3 md:grid-cols-3">
            <div className="md:col-span-2 space-y-2">
              <Field label="Wallet Address">
                <div className="flex gap-2">
                  <Input 
                    value={inputAddr} 
                    onChange={(e) => setInputAddr(e.target.value)} 
                    placeholder="Enter your wallet address (i...)" 
                    className="w-96"
                  />
                  <Button onClick={onConnectByAddress} className="min-w-[120px] whitespace-nowrap">Connect</Button>
                </div>
              </Field>
            </div>

            {/* Create / Import */}
            <div className="space-y-2">
              <Field label="New / Import">
                <Button onClick={() => setCreateImportOpen(true)} className="w-full">
                  Create / Import
                </Button>
              </Field>
            </div>
          </div>

          {hasWallet && (
            <div className="flex items-center justify-between gap-3 flex-wrap">
              <div className="flex items-center gap-3 flex-wrap">
                <span className="text-sm text-gray-600">Connected:</span>
                <span className="font-mono break-all text-sm max-w-xs">{address}</span>
                <Badge variant={walletType === "watch-only" ? "warning" : "success"}>
                  {walletType === "watch-only" ? "Watch-only" : "Local"}
                </Badge>
                <Button onClick={() => copy(address!)} className="bg-gray-600 hover:bg-gray-700 min-w-[60px] whitespace-nowrap">
                  Copy
                </Button>
                <Button onClick={() => setRecvOpen(true)} className="bg-gray-600 hover:bg-gray-700 min-w-[70px] whitespace-nowrap">
                  Receive
                </Button>
              </div>
              <div className="flex items-center gap-3">
                <a className="text-sm inline-flex items-center gap-1 underline" href={`/explorer/accounts?addr=${address}`}>
                  View in Explorer
                </a>
                <Button onClick={onDisconnect} className="bg-red-600 hover:bg-red-700 min-w-[80px] whitespace-nowrap">
                  Disconnect
                </Button>
              </div>
            </div>
          )}
        </div>
      </Card>

      {/* Real-time Node Status */}
      <Card title="IPPAN Node Status">
        <div className="space-y-4">
          {nodeData.loading ? (
            <p className="text-sm text-gray-600">Loading node data...</p>
          ) : nodeData.error ? (
            <div className="text-sm text-red-600">
              <p>Error: {nodeData.error}</p>
              <p className="text-xs mt-1">Make sure the IPPAN node is running on http://188.245.97.41:3000</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              {nodeData.status && (
                <div className="rounded-xl border p-3">
                  <div className="text-sm text-gray-600 mb-2">Node Status</div>
                  <div className="text-lg font-medium">{nodeData.status.status}</div>
                  <div className="text-xs text-gray-500 mt-1">
                    Block: {nodeData.status.current_block?.toLocaleString() || 'N/A'}
                  </div>
                  <div className="text-xs text-gray-500">
                    Uptime: {Math.floor((nodeData.status.uptime_seconds || 0) / 3600)}h
                  </div>
                </div>
              )}
              
              {nodeData.network && (
                <div className="rounded-xl border p-3">
                  <div className="text-sm text-gray-600 mb-2">Network</div>
                  <div className="text-lg font-medium">{nodeData.network.connected_peers || 0} peers</div>
                  <div className="text-xs text-gray-500 mt-1">
                    Total: {nodeData.network.total_peers || 0} peers
                  </div>
                  <div className="text-xs text-gray-500">
                    ID: {nodeData.network.network_id?.substring(0, 8) || 'N/A'}...
                  </div>
                </div>
              )}
              
              {nodeData.mempool && (
                <div className="rounded-xl border p-3">
                  <div className="text-sm text-gray-600 mb-2">Mempool</div>
                  <div className="text-lg font-medium">{nodeData.mempool.total_transactions || 0} txs</div>
                  <div className="text-xs text-gray-500 mt-1">
                    Senders: {nodeData.mempool.total_senders || 0}
                  </div>
                  <div className="text-xs text-gray-500">
                    Size: {nodeData.mempool.total_size || 0} bytes
                  </div>
                </div>
              )}
              
              {nodeData.consensus && (
                <div className="rounded-xl border p-3">
                  <div className="text-sm text-gray-600 mb-2">Consensus</div>
                  <div className="text-lg font-medium">Round {nodeData.consensus.current_round || 0}</div>
                  <div className="text-xs text-gray-500 mt-1">
                    Validators: {nodeData.consensus.validators_count || 0}
                  </div>
                  <div className="text-xs text-gray-500">
                    Status: {nodeData.consensus.consensus_status || 'N/A'}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </Card>

      {/* Balances & Actions */}
      <div className="grid md:grid-cols-3 gap-4">
                 <Card title="Balances">
           <div className="space-y-4">
             {!hasWallet ? (
               <p className="text-sm text-gray-600">Connect a wallet to load balances.</p>
             ) : loading ? (
               <p className="text-sm">Loading…</p>
             ) : (
               <>
                 <div className="text-center">
                   <div className="text-3xl font-semibold">{mainBalance.toLocaleString()} <span className="text-base font-normal">IPN</span></div>
                 </div>
                 <hr className="border-gray-200" />
                 <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                   {state?.assets.map(a => (
                     <div key={a.symbol} className="rounded-xl border p-3">
                       <div className="flex items-center justify-between mb-2">
                         <div className="text-sm text-gray-600">{a.name}</div>
                         <Badge variant={a.symbol === "IPN" ? "default" : "warning"}>{a.symbol}</Badge>
                       </div>
                       <div className="text-xl font-medium">{a.balance.toLocaleString()}</div>
                     </div>
                   ))}
                 </div>
               </>
             )}
           </div>
         </Card>

                 <Card title="Quick Actions">
           <div className="space-y-4">
             <div className="space-y-3">
               <Button onClick={() => setSendOpen(true)} className="w-full">
                 Send
               </Button>
               <Button onClick={() => setRecvOpen(true)} className="w-full bg-gray-600 hover:bg-gray-700">
                 Receive
               </Button>
             </div>
 
             <hr className="border-gray-200" />
 
             {/* Address Book Quick Pay */}
             <div className="space-y-3">
               <Field label="Address Book">
                 <Input 
                   placeholder="Search contact…" 
                   value={addrBookSearch}
                   onChange={(e) => setAddrBookSearch(e.target.value)}
                   onFocus={() => setSelectedContact(null)}
                 />
                 <div className="max-h-32 overflow-y-auto border rounded mt-1">
                   {filteredContacts.length === 0 ? (
                     <div className="p-2 text-sm text-gray-500">No contacts.</div>
                   ) : (
                     filteredContacts.map(c => (
                       <div 
                         key={c.address} 
                         className={`p-2 hover:bg-gray-50 cursor-pointer border-b last:border-b-0 ${
                           selectedContact === c.address ? 'bg-blue-100 border-blue-300' : ''
                         }`}
                         onClick={(e) => {
                           e.stopPropagation();
                           setSendTo(c.address);
                           setSelectedContact(c.address);
                         }}
                       >
                         <div className="font-medium">{c.name}</div>
                         <div className="font-mono text-xs text-gray-500 break-all">{c.address}</div>
                       </div>
                     ))
                   )}
                 </div>
               </Field>
                               <div className="space-y-2">
                  <Input 
                    placeholder="Name" 
                    value={newContactName}
                    onChange={(e) => setNewContactName(e.target.value)}
                    className="w-full"
                  />
                  <Input 
                    placeholder="i..." 
                    value={newContactAddr}
                    onChange={(e) => setNewContactAddr(e.target.value)}
                    className="w-full"
                  />
                  <Button
                    onClick={() => {
                      const name = newContactName.trim();
                      const addr = newContactAddr.trim();
                      console.log('Add button clicked:', { name, addr, addrLength: addr.length });
                      
                      if (!name || !/^i[A-Za-z0-9]{38}$/.test(addr)) { 
                        console.log('Validation failed:', { 
                          hasName: !!name, 
                          validAddress: /^i[A-Za-z0-9]{38}$/.test(addr),
                          addressLength: addr.length,
                          addressStart: addr.substring(0, 5)
                        });
                        alert("Enter name and valid address"); 
                        return; 
                      }
                      
                      console.log('Adding to book:', { name, addr });
                      addToBook(name, addr);
                      setNewContactName("");
                      setNewContactAddr("");
                    }}
                    className="w-full"
                  >
                    Add
                  </Button>
                </div>
             </div>
           </div>
         </Card>
      </div>

      {/* Staking & Domains */}
      <div className="grid md:grid-cols-2 gap-4">
        <Card title="Staking Snapshot">
          <div className="flex items-center gap-8">
            {!hasWallet ? (
              <p className="text-sm text-gray-600">Connect wallet to view staking.</p>
            ) : loading ? (
              <p className="text-sm">Loading…</p>
            ) : (
              <>
                <div>
                  <div className="text-sm text-gray-600">Staked</div>
                  <div className="text-2xl font-semibold">{state?.staking.staked.toLocaleString()} IPN</div>
                </div>
                <div>
                  <div className="text-sm text-gray-600">Rewards</div>
                  <div className="text-2xl font-semibold">{state?.staking.rewards.toLocaleString()} IPN</div>
                </div>
                <div className="ml-auto">
                  <Button onClick={() => (window.location.href = "/staking")} className="bg-gray-600 hover:bg-gray-700">
                    Manage
                  </Button>
                </div>
              </>
            )}
          </div>
        </Card>

        <Card title="Linked Domains">
          <div className="space-y-2">
            {!hasWallet ? (
              <p className="text-sm text-gray-600">Connect wallet to view linked domains.</p>
            ) : loading ? (
              <p className="text-sm">Loading…</p>
            ) : (
              <div className="flex flex-wrap gap-2">
                {state?.domains.map(d => (
                  <Badge key={d} variant="warning" className="px-3 py-1">{d}</Badge>
                ))}
                <Button onClick={() => (window.location.href = "/domains")} className="bg-gray-600 hover:bg-gray-700">
                  Manage Domains
                </Button>
              </div>
            )}
          </div>
        </Card>
      </div>

      {/* Activity + CSV export */}
      <Card title="Recent Activity">
        <div className="flex items-center justify-between mb-4">
          <div></div>
          <Button 
            onClick={() => {
              const rows = [["Title","Subtitle","Delta (IPN)","When"]];
              (state?.activities || []).forEach(a => rows.push([a.title, a.subtitle, String(a.delta), a.when]));
              downloadCSV("ippan-activity.csv", rows);
            }}
            className="bg-gray-600 hover:bg-gray-700"
          >
            Export CSV
          </Button>
        </div>
        <div>
          {!hasWallet ? (
            <p className="text-sm text-gray-600">Connect wallet to see activity.</p>
          ) : loading ? (
            <p className="text-sm">Loading…</p>
          ) : (
            <div className="divide-y">
              {state?.activities.map(a => (
                <div key={a.id} className="py-3 flex items-center justify-between">
                  <div>
                    <div className="font-medium">{a.title}</div>
                    <div className="text-sm text-gray-600">{a.subtitle}</div>
                  </div>
                  <div className={`font-medium ${a.delta >= 0 ? "text-green-600" : "text-red-600"}`}>
                    {a.delta >= 0 ? "+" : ""}{a.delta} IPN
                  </div>
                  <div className="text-sm text-gray-600 w-40 text-right">{a.when}</div>
                </div>
              ))}
            </div>
          )}
        </div>
      </Card>

      {/* Security Center */}
      <Card title="Security Center">
        <div className="space-y-4">
          <div className="grid md:grid-cols-3 gap-4">
            <div className="rounded-xl border p-3">
              <div className="text-sm font-medium">Spending Limit</div>
              <div className="text-xs text-gray-600 mb-2">Warn when a single payment exceeds this limit.</div>
              <div className="flex gap-2 items-center">
                <Input 
                  className="w-28" 
                  type="number" 
                  value={dailyLimit || ""} 
                  onChange={(e)=> setDailyLimit(Number(e.target.value) || 0)} 
                  placeholder="IPN" 
                />
                <Button onClick={()=> saveLimit(dailyLimit)} className="bg-gray-600 hover:bg-gray-700">
                  Save
                </Button>
              </div>
            </div>

            <div className="rounded-xl border p-3">
              <div className="text-sm font-medium">Devices</div>
              <div className="text-xs text-gray-600 mb-2">Approved devices with recent access.</div>
              <div className="space-y-2">
                {devices.map(d => (
                  <div key={d.id} className="flex items-center justify-between">
                    <div>
                      <div className="text-sm">{d.name}</div>
                      <div className="text-xs text-gray-600">Last: {d.last}</div>
                    </div>
                    <Button 
                      onClick={() => setDevices(prev => prev.filter(x => x.id !== d.id))}
                      className="bg-red-600 hover:bg-red-700"
                    >
                      Revoke
                    </Button>
                  </div>
                ))}
              </div>
            </div>

            <div className="rounded-xl border p-3">
              <div className="text-sm font-medium">Session</div>
              <div className="text-xs text-gray-600 mb-2">Clear local data and disconnect.</div>
              <Button 
                onClick={() => { resetSession(); onDisconnect(); }}
                className="bg-red-600 hover:bg-red-700"
              >
                Reset Session
              </Button>
            </div>
          </div>
        </div>
      </Card>

      {/* Send Drawer */}
      {sendOpen && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-2xl max-h-[90vh] overflow-y-auto">
            <div className="flex justify-between items-center mb-4">
              <h3 className="text-lg font-semibold">Send Payment</h3>
              <Button onClick={() => setSendOpen(false)} className="bg-gray-600 hover:bg-gray-700">
                ✕
              </Button>
            </div>
            <div className="space-y-3">
              <Field label="To">
                <div className="flex gap-2">
                  <Input 
                    value={sendTo} 
                    onChange={(e) => { setSendTo(e.target.value); setCheckingRecipient("idle"); setSelectedContact(null); }} 
                    placeholder="i..." 
                    className="w-96"
                  />
                  <Button onClick={checkRecipient} className="bg-gray-600 hover:bg-gray-700 min-w-[80px] whitespace-nowrap">
                    {checkingRecipient === "checking" ? "Checking…" : checkingRecipient === "ok" ? "OK" : "Check"}
                  </Button>
                </div>
                {checkingRecipient === "bad" && <p className="text-xs text-red-600">Address not found in Explorer.</p>}
              </Field>

              <Field label="Amount (IPN)">
                <Input 
                  value={sendAmount} 
                  onChange={(e) => setSendAmount(e.target.value)} 
                  placeholder="0.00" 
                />
              </Field>

              <Field label="Memo (optional)">
                <Input 
                  value={sendMemo} 
                  onChange={(e) => setSendMemo(e.target.value)} 
                  placeholder="note…" 
                />
              </Field>

              {/* Address Book Quick Pick */}
              <Field label="Address Book">
                <Input 
                  placeholder="Search contact…" 
                  value={addrBookSearch}
                  onChange={(e) => setAddrBookSearch(e.target.value)}
                  onFocus={() => setSelectedContact(null)}
                />
                <div className="max-h-32 overflow-y-auto border rounded mt-1">
                  {filteredContacts.length === 0 ? (
                    <div className="p-2 text-sm text-gray-500">No contacts.</div>
                  ) : (
                    filteredContacts.map(c => (
                      <div 
                        key={c.address} 
                        className={`p-2 hover:bg-gray-50 cursor-pointer border-b last:border-b-0 ${
                          selectedContact === c.address ? 'bg-blue-100 border-blue-300' : ''
                        }`}
                        onClick={(e) => {
                          e.stopPropagation();
                          setSendTo(c.address);
                          setSelectedContact(c.address);
                        }}
                      >
                        <div className="font-medium">{c.name}</div>
                        <div className="font-mono text-xs text-gray-500 break-all">{c.address}</div>
                      </div>
                    ))
                  )}
                </div>
              </Field>

              {/* Advanced */}
              <div className="rounded-lg border p-3">
                <div className="text-sm font-medium mb-2">Advanced</div>
                <div className="grid grid-cols-1 gap-3">
                  <div>
                    <label className="text-sm">Priority</label>
                    <div className="flex items-center gap-3">
                      <input 
                        type="range" 
                        min="0" 
                        max="1" 
                        step="0.05" 
                        value={priority}
                        onChange={(e) => setPriority(Number(e.target.value))}
                        className="w-56"
                      />
                      <span className="text-sm text-gray-600">{Math.round(priority*100)}%</span>
                    </div>
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <div className="text-xs text-gray-600">Estimated Fee</div>
                      <div className="text-sm">{feePreview ? `${feePreview.fee} IPN` : "—"}</div>
                    </div>
                    <div>
                      <div className="text-xs text-gray-600">Max Fee</div>
                      <div className="text-sm">{feePreview ? `${feePreview.maxFee} IPN` : "—"}</div>
                    </div>
                    <div>
                      <div className="text-xs text-gray-600">Nonce</div>
                      <div className="flex gap-2">
                        <Input 
                          className="w-28" 
                          placeholder={feePreview ? String(feePreview.nonce) : "—"} 
                          value={nonceOverride} 
                          onChange={(e)=>setNonceOverride(e.target.value)} 
                        />
                      </div>
                    </div>
                    <div>
                      <div className="text-xs text-gray-600">ETA</div>
                      <div className="text-sm">{feePreview ? `${feePreview.etaSeconds}s` : "—"}</div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            <div className="flex gap-2 mt-4">
              <Button onClick={() => setSendOpen(false)} className="bg-gray-600 hover:bg-gray-700">
                Cancel
              </Button>
              <Button onClick={onSend} disabled={!address || !sendTo || !sendAmount}>
                Send
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Receive Modal */}
      {recvOpen && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-md">
            <h3 className="text-lg font-semibold mb-4">Receive IPN</h3>
            <div className="space-y-3">
              <p className="text-sm text-gray-600">Share this address to receive funds:</p>
              <div className="p-3 rounded-md border bg-gray-100 font-mono text-sm break-all">
                {address ?? "—"}
              </div>
              <Button onClick={() => copy(address || "")} className="bg-gray-600 hover:bg-gray-700">
                Copy Address
              </Button>
              {qr ? (
                <img src={qr} alt="QR" className="rounded-md border p-2 max-w-xs" />
              ) : (
                <div className="rounded-md border p-10 text-center text-sm text-gray-600">
                  QR will appear here
                </div>
              )}
            </div>
            <div className="mt-4">
              <Button onClick={() => setRecvOpen(false)}>Close</Button>
            </div>
          </div>
        </div>
      )}

      {/* Create/Import Modal */}
      {createImportOpen && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-2xl">
            <h3 className="text-lg font-semibold mb-4">New Account or Import</h3>
            
            {/* Tabs */}
            <div className="flex border-b mb-4">
              <button
                onClick={() => setActiveTab("create")}
                className={`px-4 py-2 border-b-2 font-medium text-sm ${
                  activeTab === "create"
                    ? "border-blue-500 text-blue-600"
                    : "border-transparent text-gray-500 hover:text-gray-700"
                }`}
              >
                Create
              </button>
              <button
                onClick={() => setActiveTab("import")}
                className={`px-4 py-2 border-b-2 font-medium text-sm ${
                  activeTab === "import"
                    ? "border-blue-500 text-blue-600"
                    : "border-transparent text-gray-500 hover:text-gray-700"
                }`}
              >
                Import
              </button>
            </div>

            {/* Create Tab */}
            {activeTab === "create" && (
              <div className="space-y-3">
                <p className="text-sm text-gray-600">Save this seed phrase securely. Anyone with it can control your funds.</p>
                <div className="p-3 rounded-md border bg-gray-100">
                  <code className="text-sm break-words">
                    {revealSeed ? seed : "•••• •••• •••• •••• •••• ••••"}
                  </code>
                </div>
                <div className="flex items-center gap-3">
                  <input
                    type="checkbox"
                    checked={revealSeed}
                    onChange={(e) => setRevealSeed(e.target.checked)}
                    className="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500"
                  />
                  <span className="text-sm">{revealSeed ? "Hide seed" : "Reveal seed"}</span>
                  <Button onClick={() => setSeed(generateSeed(12))} className="bg-gray-600 hover:bg-gray-700">
                    Regenerate
                  </Button>
                  <Button onClick={() => navigator.clipboard.writeText(seed)} className="bg-gray-600 hover:bg-gray-700">
                    Copy
                  </Button>
                  <Button
                    onClick={() => {
                      const blob = new Blob([seed], { type: "text/plain" });
                      const url = URL.createObjectURL(blob);
                      const a = document.createElement("a");
                      a.href = url; a.download = "ippan-seed.txt"; a.click();
                      URL.revokeObjectURL(url);
                    }}
                    className="bg-gray-600 hover:bg-gray-700"
                  >
                    Download
                  </Button>
                </div>
                                 <Button
                   onClick={async () => {
                     const addr = await addressFromSeed(seed);
                     setConnected(addr, "local");
                     setInputAddr(addr);
                     setCreateImportOpen(false);
                   }}
                 >
                   Use This Seed
                 </Button>
              </div>
            )}

            {/* Import Tab */}
            {activeTab === "import" && (
              <div className="space-y-3">
                <Field label="Mnemonic (12/24 words) or Private Key (i...)">
                  <Input 
                    placeholder="paste mnemonic or private key…" 
                    value={priv} 
                    onChange={(e) => setPriv(e.target.value)} 
                  />
                </Field>
                <div className="flex gap-2">
                                     <Button
                     onClick={async () => {
                       const src = priv.trim();
                       if (!src) return;
                       const addr = src.startsWith("0x") ? await addressFromPrivKey(src) : await addressFromSeed(src);
                       setConnected(addr, "local"); setInputAddr(addr);
                       setCreateImportOpen(false);
                     }}
                   >
                     Import
                   </Button>
                  <Button onClick={() => setPriv("")} className="bg-gray-600 hover:bg-gray-700">
                    Clear
                  </Button>
                </div>
              </div>
            )}

            <div className="mt-4">
              <Button onClick={() => setCreateImportOpen(false)} className="bg-gray-600 hover:bg-gray-700">
                Cancel
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
