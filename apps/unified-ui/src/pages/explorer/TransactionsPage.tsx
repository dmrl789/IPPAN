import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { Card, Button, Badge, LoadingSpinner, Field, Input } from '../../components/UI'

// Canonical IPPAN HashTimer v1 structure matching JSON schema
interface HashTimer {
  version: 'v1';
  time: {
    t_ns: string;              // Nanoseconds since epoch (stringified big int)
    precision_ns: number;      // Precision quantum
    drift_ns: string;          // Clock drift (stringified signed int)
  };
  position: {
    round: string;             // Consensus round (stringified big int)
    seq: number;               // Sequence number within round
    kind: 'Tx' | 'Block' | 'Round';
  };
  node_id: string;             // 16-byte node identifier (32 hex chars)
  payload_digest: string;      // SHA-256 of event payload (64 hex chars)
  hash_timer_digest: string;   // SHA-256 of 96-byte HashTimer buffer (64 hex chars)
}

// Canonical IPPAN Transaction structure matching JSON schema
interface Transaction {
  tx_hash: string;             // Transaction hash (64 hex chars)
  from: string;                // Sender address (32 hex chars for flexibility)
  to: string;                  // Recipient address (32 hex chars for flexibility)
  amount: string;              // Amount (stringified big int)
  fee: number;                 // Transaction fee
  nonce: number;               // Transaction nonce
  memo?: string;               // Optional memo
  signature: string;           // Transaction signature
  hashtimer: HashTimer;        // Canonical HashTimer v1
  block_id?: string;           // Block containing this transaction
  block_parents?: string[];    // Parent blocks of the containing block
  block_parent_rounds?: string[]; // Parent rounds of the containing block
  confidentiality?: 'public' | 'private' | 'confidential'; // Transaction privacy level
}

// Helper function to create canonical IPPAN HashTimer v1
const createHashTimer = (timestamp: number, nodeId: string, round: number, sequence: number, kind: 'Tx' | 'Block' | 'Round', payloadDigest?: string): HashTimer => {
  const timestamp_ns = timestamp * 1_000_000; // Convert to nanoseconds
  const drift_ns = Math.floor(Math.random() * 1000) - 500; // Random drift ±500ns
  
  // Generate 16-byte node_id (32 hex chars)
  const nodeIdBytes = new TextEncoder().encode(nodeId);
  const nodeHash = sha256(nodeIdBytes);
  const nodeIdHex = Array.from(nodeHash.slice(0, 16)).map(b => b.toString(16).padStart(2, '0')).join('');
  
  // Generate payload digest if not provided
  const finalPayloadDigest = payloadDigest || Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join('');
  
  // Create 96-byte HashTimer input buffer (canonical encoding)
  const buffer = new ArrayBuffer(96);
  const view = new DataView(buffer);
  
  // Version tag (16 bytes)
  const versionTag = new TextEncoder().encode("IPPAN-HT-v1____");
  for (let i = 0; i < 16; i++) {
    view.setUint8(i, i < versionTag.length ? versionTag[i] : 0);
  }
  
  // Time (8 bytes, little-endian)
  view.setBigUint64(16, BigInt(timestamp_ns), true);
  
  // Precision (4 bytes, little-endian)
  view.setUint32(24, 100, true);
  
  // Drift (4 bytes, little-endian, signed)
  view.setInt32(28, drift_ns, true);
  
  // Round (8 bytes, little-endian)
  view.setBigUint64(32, BigInt(round), true);
  
  // Sequence (4 bytes, little-endian)
  view.setUint32(40, sequence, true);
  
  // Kind (4 bytes, little-endian) - 1=Tx, 2=Block, 3=Round
  const kindValue = kind === 'Tx' ? 1 : kind === 'Block' ? 2 : 3;
  view.setUint32(44, kindValue, true);
  
  // Node ID (16 bytes)
  const nodeIdBytes2 = new Uint8Array(16);
  for (let i = 0; i < 16; i++) {
    nodeIdBytes2[i] = parseInt(nodeIdHex.substr(i * 2, 2), 16);
  }
  for (let i = 0; i < 16; i++) {
    view.setUint8(48 + i, nodeIdBytes2[i]);
  }
  
  // Payload digest (32 bytes)
  for (let i = 0; i < 32; i++) {
    const byte = parseInt(finalPayloadDigest.substr(i * 2, 2), 16);
    view.setUint8(64 + i, byte);
  }
  
  // Calculate hash_timer_digest (SHA-256 of the 96-byte buffer)
  const bufferBytes = new Uint8Array(buffer);
  const hashTimerDigest = sha256(bufferBytes);
  const hashTimerDigestHex = Array.from(hashTimerDigest).map(b => b.toString(16).padStart(2, '0')).join('');
  
  return {
    version: 'v1',
    time: {
      t_ns: timestamp_ns.toString(),
      precision_ns: 100,
      drift_ns: drift_ns.toString()
    },
    position: {
      round: round.toString(),
      seq: sequence,
      kind
    },
    node_id: nodeIdHex,
    payload_digest: finalPayloadDigest,
    hash_timer_digest: hashTimerDigestHex
  };
};

// Mock SHA-256 function (in real implementation, use crypto.subtle.digest)
const sha256 = (_data: Uint8Array): Uint8Array => {
  // This is a mock implementation - in production, use proper SHA-256
  const hash = new Uint8Array(32);
  for (let i = 0; i < 32; i++) {
    hash[i] = Math.floor(Math.random() * 256);
  }
  return hash;
};

export default function TransactionsPage() {
  const navigate = useNavigate()
  const [transactions, setTransactions] = useState<Transaction[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [selectedTx, setSelectedTx] = useState<Transaction | null>(null)
  const [searchTerm, setSearchTerm] = useState('')
  const [filterType, setFilterType] = useState<string>('all')
  const [currentPage, setCurrentPage] = useState(1)
  const [itemsPerPage] = useState(20)

  // Helper function to create canonical IPPAN transactions
  const createTransaction = (blockId: string, txIndex: number, timestamp: number, nodeId: string, round: number, sequence: number): Transaction => {
    const txHash = Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join('');
    const from = Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join('');
    const to = Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join('');
    const amount = (Math.floor(Math.random() * 1000000) + 1000).toString();
    const fee = Math.floor(Math.random() * 1000) + 100;
    const nonce = Math.floor(Math.random() * 10000) + 1;
    const signature = Array.from({length: 128}, () => Math.floor(Math.random() * 16).toString(16)).join('');
    
    // Randomly assign confidentiality level (70% public, 20% private, 10% confidential)
    const confidentialityRand = Math.random();
    const confidentiality: 'public' | 'private' | 'confidential' = 
      confidentialityRand < 0.7 ? 'public' : 
      confidentialityRand < 0.9 ? 'private' : 'confidential';
    
    // Generate block parents for context
    const blockParents = [
      Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join(''),
      Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join('')
    ];
    const blockParentRounds = [(round - 1).toString(), (round - 2).toString()];
    
    const hashtimer = createHashTimer(timestamp, nodeId, round, sequence, 'Tx');
    
    // Generate appropriate memo based on confidentiality
    let memo = '';
    if (confidentiality === 'public') {
      memo = `Transaction ${txIndex} in block ${blockId}`;
    } else if (confidentiality === 'private') {
      memo = '[Private Transaction]';
    } else {
      memo = '[Confidential Transaction]';
    }
    
    return {
      tx_hash: txHash,
      from,
      to,
      amount,
      fee,
      nonce,
      memo,
      signature,
      hashtimer,
      block_id: blockId,
      block_parents: blockParents,
      block_parent_rounds: blockParentRounds,
      confidentiality
    };
  };

  // Mock data for demonstration
  useEffect(() => {
    const now = Date.now();
    const mockTransactions: Transaction[] = [];
    
    // Generate transactions across multiple blocks and rounds
    for (let blockIndex = 0; blockIndex < 5; blockIndex++) {
      const blockId = `block-${1234560 + blockIndex}`;
      const blockTimestamp = now - (blockIndex * 10000);
      const round = 8784975000 + blockIndex;
      const nodeId = `validator-${(blockIndex % 3) + 1}`;
      
      // Generate 3-8 transactions per block
      const txCount = Math.floor(Math.random() * 6) + 3;
      for (let txIndex = 0; txIndex < txCount; txIndex++) {
        const txTimestamp = blockTimestamp + (txIndex * 1000);
        const transaction = createTransaction(blockId, txIndex, txTimestamp, nodeId, round, txIndex + 1);
        mockTransactions.push(transaction);
      }
    }
    
    // Sort transactions by HashTimer (canonical ordering)
    mockTransactions.sort((a, b) => {
      const aTime = BigInt(a.hashtimer.time.t_ns);
      const bTime = BigInt(b.hashtimer.time.t_ns);
      if (aTime !== bTime) return aTime < bTime ? -1 : 1;
      
      const aRound = BigInt(a.hashtimer.position.round);
      const bRound = BigInt(b.hashtimer.position.round);
      if (aRound !== bRound) return aRound < bRound ? -1 : 1;
      
      if (a.hashtimer.position.seq !== b.hashtimer.position.seq) {
        return a.hashtimer.position.seq - b.hashtimer.position.seq;
      }
      
      if (a.hashtimer.node_id !== b.hashtimer.node_id) {
        return a.hashtimer.node_id.localeCompare(b.hashtimer.node_id);
      }
      
      return a.hashtimer.payload_digest.localeCompare(b.hashtimer.payload_digest);
    });

    setTransactions(mockTransactions)
    setIsLoading(false)
  }, [])

  // Filter transactions based on search and type
  const filteredTransactions = transactions.filter(tx => {
    const matchesSearch = 
      tx.tx_hash.toLowerCase().includes(searchTerm.toLowerCase()) ||
      tx.from.toLowerCase().includes(searchTerm.toLowerCase()) ||
      tx.to.toLowerCase().includes(searchTerm.toLowerCase()) ||
      tx.amount.toLowerCase().includes(searchTerm.toLowerCase()) ||
      tx.block_id?.toLowerCase().includes(searchTerm.toLowerCase()) ||
      tx.hashtimer.position.round.includes(searchTerm)
    
    // For now, we'll use a simple type filter based on amount or memo
    const matchesType = filterType === 'all' || 
      (filterType === 'transfer' && parseInt(tx.amount) > 0) ||
      (filterType === 'stake' && tx.memo?.includes('stake')) ||
      (filterType === 'contract' && tx.memo?.includes('contract')) ||
      (filterType === 'ai_job' && tx.memo?.includes('AI'))
    
    return matchesSearch && matchesType
  })

  // Pagination
  const totalPages = Math.ceil(filteredTransactions.length / itemsPerPage)
  const startIndex = (currentPage - 1) * itemsPerPage
  const paginatedTransactions = filteredTransactions.slice(startIndex, startIndex + itemsPerPage)

  const getStatusColor = (tx: Transaction) => {
    // Determine status based on HashTimer and block info
    const now = BigInt(Date.now() * 1_000_000); // Convert to nanoseconds
    const txTime = BigInt(tx.hashtimer.time.t_ns);
    const timeDiff = Number(now - txTime);
    
    if (timeDiff < 30_000_000_000) { // Less than 30 seconds
      return 'warning'; // pending
    } else if (tx.block_id) {
      return 'success'; // confirmed
    } else {
      return 'error'; // failed
    }
  }

  const getTypeColor = (tx: Transaction) => {
    if (parseInt(tx.amount) === 0) return 'purple'; // contract
    if (tx.memo?.includes('stake')) return 'green'; // stake
    if (tx.memo?.includes('AI')) return 'pink'; // ai_job
    return 'blue'; // transfer
  }

  const getTransactionType = (tx: Transaction) => {
    if (parseInt(tx.amount) === 0) return 'contract';
    if (tx.memo?.includes('stake')) return 'stake';
    if (tx.memo?.includes('AI')) return 'ai_job';
    return 'transfer';
  }

  const getConfidentialityColor = (confidentiality?: string) => {
    switch (confidentiality) {
      case 'public': return 'default';
      case 'private': return 'warning';
      case 'confidential': return 'error';
      default: return 'default';
    }
  }

  const getConfidentialityIcon = (confidentiality?: string) => {
    switch (confidentiality) {
      case 'public': return '🌐';
      case 'private': return '🔒';
      case 'confidential': return '🛡️';
      default: return '🌐';
    }
  }

  const handleViewBlock = () => {
    if (selectedTx?.block_id) {
      // Navigate to Live Blocks page with block ID as a query parameter
      navigate(`/explorer/live-blocks?block=${selectedTx.block_id}`)
    }
  }

  if (isLoading) {
    return (
      <div className="flex justify-center items-center h-64">
        <LoadingSpinner />
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold text-gray-900">Transactions</h1>
        <Badge variant="success">Live Network</Badge>
      </div>

      {/* Search and Filters */}
      <Card title="Search & Filters">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <Field label="Search Transactions">
            <Input
              placeholder="Search by hash, address, or amount..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
            />
          </Field>
          <Field label="Transaction Type">
            <select
              value={filterType}
              onChange={(e) => setFilterType(e.target.value)}
              className="w-full p-2 border border-gray-300 rounded-md"
            >
              <option value="all">All Types</option>
              <option value="transfer">Transfer</option>
              <option value="stake">Stake</option>
              <option value="unstake">Unstake</option>
              <option value="contract">Smart Contract</option>
              <option value="ai_job">AI Job</option>
            </select>
          </Field>
          <div className="flex items-end">
            <Button className="w-full">Search</Button>
          </div>
        </div>
      </Card>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Transaction List */}
        <div className="lg:col-span-2">
          <Card title={`Transactions (${filteredTransactions.length})`}>
            <div className="space-y-3">
              {paginatedTransactions.map((tx) => (
                <div
                  key={tx.tx_hash}
                  className={`p-4 border rounded-lg cursor-pointer transition-all hover:shadow-md ${
                    selectedTx?.tx_hash === tx.tx_hash ? 'border-blue-500 bg-blue-50' : 'border-gray-200'
                  }`}
                  onClick={() => setSelectedTx(tx)}
                >
                  <div className="flex justify-between items-start">
                    <div className="flex-1">
                      <div className="flex items-center space-x-2 mb-2">
                        <Badge variant={getTypeColor(tx) as any}>
                          {getTransactionType(tx).replace('_', ' ').toUpperCase()}
                        </Badge>
                        <Badge variant={getStatusColor(tx) as any}>
                          {getStatusColor(tx) === 'warning' ? 'pending' : 
                           getStatusColor(tx) === 'success' ? 'confirmed' : 'failed'}
                        </Badge>
                        <Badge variant={getConfidentialityColor(tx.confidentiality) as any}>
                          {getConfidentialityIcon(tx.confidentiality)} {tx.confidentiality?.toUpperCase() || 'PUBLIC'}
                        </Badge>
                        <span className="text-sm text-gray-600">Block {tx.block_id}</span>
                        <span className="text-sm text-gray-600">Round {tx.hashtimer.position.round}</span>
                      </div>
                      <div className="text-sm text-gray-600 space-y-1">
                        <div>Hash: {tx.tx_hash.substring(0, 16)}...</div>
                        <div>From: {tx.confidentiality === 'public' ? `${tx.from.substring(0, 16)}...` : 
                                   tx.confidentiality === 'private' ? '[Encrypted]' : '[Confidential]'}</div>
                        <div>To: {tx.confidentiality === 'public' ? `${tx.to.substring(0, 16)}...` : 
                                 tx.confidentiality === 'private' ? '[Encrypted]' : '[Confidential]'}</div>
                        <div>Amount: {tx.confidentiality === 'public' ? `${tx.amount} IPPAN` : 
                                     tx.confidentiality === 'private' ? '[Encrypted]' : '[Confidential]'}</div>
                        <div>HashTimer: {tx.hashtimer.time.t_ns}ns</div>
                      </div>
                    </div>
                    <div className="text-right text-sm text-gray-600">
                      <div>{tx.confidentiality === 'public' ? `${tx.amount} IPPAN` : 
                             tx.confidentiality === 'private' ? '[Encrypted]' : '[Confidential]'}</div>
                      <div>Fee: {tx.fee}</div>
                      <div>{new Date(parseInt(tx.hashtimer.time.t_ns) / 1_000_000).toLocaleTimeString()}</div>
                    </div>
                  </div>
                </div>
              ))}
            </div>

            {/* Pagination */}
            {totalPages > 1 && (
              <div className="flex justify-center items-center space-x-2 mt-6">
                <Button
                  onClick={() => setCurrentPage(Math.max(1, currentPage - 1))}
                  disabled={currentPage === 1}
                  className="px-3 py-1"
                >
                  Previous
                </Button>
                <span className="text-sm text-gray-600">
                  Page {currentPage} of {totalPages}
                </span>
                <Button
                  onClick={() => setCurrentPage(Math.min(totalPages, currentPage + 1))}
                  disabled={currentPage === totalPages}
                  className="px-3 py-1"
                >
                  Next
                </Button>
              </div>
            )}
          </Card>
        </div>

        {/* Transaction Details */}
        <div className="lg:col-span-1">
          <Card title="Transaction Details">
            {selectedTx ? (
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700">Transaction Hash</label>
                  <div className="font-mono text-xs break-all">{selectedTx.tx_hash}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Block ID</label>
                  <div className="font-mono text-lg">{selectedTx.block_id}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">From Address</label>
                  <div className="font-mono text-xs break-all">
                    {selectedTx.confidentiality === 'public' ? selectedTx.from : 
                     selectedTx.confidentiality === 'private' ? '[Encrypted Address]' : 
                     '[Confidential Address]'}
                  </div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">To Address</label>
                  <div className="font-mono text-xs break-all">
                    {selectedTx.confidentiality === 'public' ? selectedTx.to : 
                     selectedTx.confidentiality === 'private' ? '[Encrypted Address]' : 
                     '[Confidential Address]'}
                  </div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Amount</label>
                  <div className="text-lg font-bold">
                    {selectedTx.confidentiality === 'public' ? `${selectedTx.amount} IPPAN` : 
                     selectedTx.confidentiality === 'private' ? '[Encrypted Amount]' : 
                     '[Confidential Amount]'}
                  </div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Fee</label>
                  <div className="text-sm">{selectedTx.fee}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Nonce</label>
                  <div className="text-sm">{selectedTx.nonce}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Memo</label>
                  <div className="text-sm">{selectedTx.memo || 'None'}</div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Type</label>
                  <Badge variant={getTypeColor(selectedTx) as any}>
                    {getTransactionType(selectedTx).replace('_', ' ').toUpperCase()}
                  </Badge>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Status</label>
                  <Badge variant={getStatusColor(selectedTx) as any}>
                    {getStatusColor(selectedTx) === 'warning' ? 'pending' : 
                     getStatusColor(selectedTx) === 'success' ? 'confirmed' : 'failed'}
                  </Badge>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">Confidentiality</label>
                  <Badge variant={getConfidentialityColor(selectedTx.confidentiality) as any}>
                    {getConfidentialityIcon(selectedTx.confidentiality)} {selectedTx.confidentiality?.toUpperCase() || 'PUBLIC'}
                  </Badge>
                  <div className="text-xs text-gray-500 mt-1">
                    {selectedTx.confidentiality === 'public' && 'Transaction details are publicly visible'}
                    {selectedTx.confidentiality === 'private' && 'Transaction details are encrypted and private'}
                    {selectedTx.confidentiality === 'confidential' && 'Transaction uses advanced confidentiality protocols'}
                  </div>
                </div>
                
                {/* HashTimer Details */}
                <div className="border-t pt-4">
                  <h4 className="font-medium text-gray-700 mb-2">HashTimer Details (v1)</h4>
                  <div className="space-y-2 text-sm">
                    <div>
                      <span className="font-medium">Time (ns):</span> {selectedTx.hashtimer.time.t_ns}
                    </div>
                    <div>
                      <span className="font-medium">Precision:</span> {selectedTx.hashtimer.time.precision_ns}ns
                    </div>
                    <div>
                      <span className="font-medium">Drift:</span> {selectedTx.hashtimer.time.drift_ns}ns
                    </div>
                    <div>
                      <span className="font-medium">Round:</span> {selectedTx.hashtimer.position.round}
                    </div>
                    <div>
                      <span className="font-medium">Sequence:</span> {selectedTx.hashtimer.position.seq}
                    </div>
                    <div>
                      <span className="font-medium">Kind:</span> {selectedTx.hashtimer.position.kind}
                    </div>
                    <div>
                      <span className="font-medium">Node ID:</span> {selectedTx.hashtimer.node_id.substring(0, 16)}...
                    </div>
                    <div>
                      <span className="font-medium">Payload Digest:</span> {selectedTx.hashtimer.payload_digest.substring(0, 16)}...
                    </div>
                    <div>
                      <span className="font-medium">HashTimer Digest:</span> {selectedTx.hashtimer.hash_timer_digest.substring(0, 16)}...
                    </div>
                  </div>
                </div>

                {/* Block Parents */}
                {selectedTx.block_parents && selectedTx.block_parents.length > 0 && (
                  <div className="border-t pt-4">
                    <h4 className="font-medium text-gray-700 mb-2">Block Parents ({selectedTx.block_parents.length})</h4>
                    <div className="space-y-2">
                      {selectedTx.block_parents.map((parent, index) => (
                        <div key={index} className="flex items-center space-x-2">
                          <span className="text-xs text-gray-500">Round {selectedTx.block_parent_rounds?.[index]}:</span>
                          <span className="text-xs font-mono bg-gray-100 px-2 py-1 rounded">
                            {parent.substring(0, 8)}...{parent.substring(56)}
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                <div>
                  <label className="block text-sm font-medium text-gray-700">Timestamp</label>
                  <div className="text-sm">{new Date(parseInt(selectedTx.hashtimer.time.t_ns) / 1_000_000).toLocaleString()}</div>
                </div>
                <Button 
                  className="w-full" 
                  onClick={handleViewBlock}
                  disabled={!selectedTx?.block_id}
                >
                  View Block
                </Button>
              </div>
            ) : (
              <div className="text-center text-gray-500 py-8">
                Select a transaction to view details
              </div>
            )}
          </Card>
        </div>
      </div>

      {/* Statistics */}
      <Card title="Transaction Statistics">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="text-center">
            <div className="text-2xl font-bold text-blue-600">{transactions.length}</div>
            <div className="text-sm text-gray-600">Total Transactions</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold text-green-600">
              {transactions.filter(tx => getStatusColor(tx) === 'success').length}
            </div>
            <div className="text-sm text-gray-600">Confirmed</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold text-yellow-600">
              {transactions.filter(tx => getStatusColor(tx) === 'warning').length}
            </div>
            <div className="text-sm text-gray-600">Pending</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold text-red-600">
              {transactions.filter(tx => getStatusColor(tx) === 'error').length}
            </div>
            <div className="text-sm text-gray-600">Failed</div>
          </div>
        </div>
      </Card>
    </div>
  )
}
