import { useState, useEffect } from 'react';
import { useSearchParams } from 'react-router-dom';

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
}

// Canonical IPPAN Block structure matching JSON schema
interface Block {
  block_id: string;            // Block identifier
  producer: {
    node_id: string;           // 16-byte node identifier (32 hex chars)
    label: string;             // Human-readable label (e.g., "validator-1")
  };
  status: 'pending' | 'finalized';
  tx_count: number;            // Transaction count
  header_digest: string;       // Block header digest (64 hex chars)
  hashtimer: HashTimer;        // Canonical HashTimer v1
  parents: string[];           // Array of parent block hashes (32-byte hex)
  parent_rounds: string[];     // Array of parent round numbers (stringified big ints)
  txs: Transaction[];          // Array of transactions
}

// Canonical IPPAN Round structure matching JSON schema
interface Round {
  version: 'v1';
  round_id: string;            // Round identifier (stringified big int)
  state: 'pending' | 'finalizing' | 'finalized' | 'rejected';
  time: {
    start_ns: string;          // Start time in nanoseconds (stringified big int)
    end_ns: string;            // End time in nanoseconds (stringified big int)
  };
  block_count: number;         // Number of blocks in this round
  zk_stark_proof: string;      // ZK-STARK proof (64 hex chars)
  merkle_root: string;         // Merkle root (64 hex chars)
  blocks: Block[];             // Array of blocks
}

interface BlockConnection {
  from: string;
  to: string;
  type: 'parent' | 'reference';
}

export default function LiveBlocksPage() {
  const [searchParams] = useSearchParams();
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [rounds, setRounds] = useState<Round[]>([]);
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [connections, setConnections] = useState<BlockConnection[]>([]);
  const [selectedBlock, setSelectedBlock] = useState<Block | null>(null);
  const [selectedRound, setSelectedRound] = useState<Round | null>(null);
  const [selectedTransaction, setSelectedTransaction] = useState<Transaction | null>(null);
  const [viewMode, setViewMode] = useState<'dag' | 'timeline' | 'rounds' | 'transactions'>('rounds');
  const [autoRefresh, setAutoRefresh] = useState(true);

  // Handle block parameter from URL
  useEffect(() => {
    const blockId = searchParams.get('block');
    if (blockId && blocks.length > 0) {
      const block = blocks.find(b => b.block_id === blockId);
      if (block) {
        setSelectedBlock(block);
        // Scroll to the block or open modal
        setTimeout(() => {
          const element = document.getElementById(`block-${blockId}`);
          if (element) {
            element.scrollIntoView({ behavior: 'smooth', block: 'center' });
          }
        }, 100);
      }
    }
  }, [searchParams, blocks]);

  // Generate mock DAG blockchain data with proper Round structure
  useEffect(() => {
    const generateMockData = () => {
      const mockBlocks: Block[] = [];
      const mockRounds: Round[] = [];
      const mockConnections: BlockConnection[] = [];
      
      const now = Date.now();
      const roundDuration = 200; // 200ms rounds as per IPPAN spec
      let currentRound = Math.floor((now - 300000) / roundDuration); // Start 5 minutes ago
      
      // Helper function to create canonical IPPAN HashTimer v1 matching JSON schema
      const createHashTimer = (timestamp: number, nodeId: string, round: number, sequence: number, kind: 'Tx' | 'Block' | 'Round', payloadDigest?: string): HashTimer => {
        const timestamp_ns = timestamp * 1_000_000; // Convert to nanoseconds
        const drift_ns = Math.floor(Math.random() * 1000) - 500; // Random drift ±500ns
        
        // Generate 16-byte node_id (32 hex chars)
        const nodeIdBytes = new TextEncoder().encode(nodeId);
        const nodeHash = sha256(nodeIdBytes);
        const nodeIdHex = Array.from(nodeHash.slice(0, 16)).map(b => b.toString(16).padStart(2, '0')).join('');
        
        // Create payload digest if not provided
        const finalPayloadDigest = payloadDigest || Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join('');
        
        // Create proper IPPAN HashTimer v1: 96-byte input -> 32-byte SHA-256 digest
        const hashTimerDigest = createCanonicalHashTimer(timestamp_ns, nodeIdHex, round, sequence, kind, finalPayloadDigest);
        
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
          hash_timer_digest: hashTimerDigest
        };
      };

      // Helper function to create canonical HashTimer v1: 96-byte input -> 32-byte SHA-256 digest
      const createCanonicalHashTimer = (timestamp_ns: number, nodeId: string, round: number, sequence: number, kind: 'Tx' | 'Block' | 'Round', payloadDigest?: string): string => {
        // Create 96-byte buffer as per canonical specification
        const buf = new Uint8Array(96);
        
        // version_tag (16 bytes): "IPPAN-HT-v1____"
        const versionTag = new TextEncoder().encode("IPPAN-HT-v1____");
        buf.set(versionTag, 0);
        
        // t_ns (8 bytes, little-endian)
        const timestampBytes = new Uint8Array(new BigUint64Array([BigInt(timestamp_ns)]).buffer);
        buf.set(timestampBytes, 16);
        
        // precision_ns (4 bytes, little-endian)
        const precisionBytes = new Uint8Array(new Uint32Array([100]).buffer);
        buf.set(precisionBytes, 24);
        
        // drift_ns (4 bytes, little-endian)
        const driftBytes = new Uint8Array(new Int32Array([Math.floor(Math.random() * 1000) - 500]).buffer);
        buf.set(driftBytes, 28);
        
        // round (8 bytes, little-endian)
        const roundBytes = new Uint8Array(new BigUint64Array([BigInt(round)]).buffer);
        buf.set(roundBytes, 32);
        
        // seq (4 bytes, little-endian)
        const seqBytes = new Uint8Array(new Uint32Array([sequence]).buffer);
        buf.set(seqBytes, 40);
        
        // kind (1 byte)
        const kindValue = kind === 'Tx' ? 0 : kind === 'Block' ? 1 : 2;
        buf[44] = kindValue;
        
        // _pad_kind (3 bytes, must be zero)
        buf[45] = 0; buf[46] = 0; buf[47] = 0;
        
        // node_id (16 bytes) - convert nodeId to 16-byte hex representation
        const nodeIdBytes = new TextEncoder().encode(nodeId);
        const nodeHash = sha256(nodeIdBytes);
        buf.set(nodeHash.slice(0, 16), 48);
        
        // payload_digest (32 bytes)
        const payloadDigestBytes = payloadDigest ? 
          new Uint8Array(payloadDigest.match(/.{2}/g)?.map(byte => parseInt(byte, 16)) || []) :
          sha256(`${timestamp_ns}_${nodeId}_${round}_${sequence}`);
        buf.set(payloadDigestBytes.slice(0, 32), 64);
        
        // Hash the 96-byte buffer with SHA-256 to get 32-byte digest
        const hashBytes = sha256(buf);
        return Array.from(hashBytes).map(b => b.toString(16).padStart(2, '0')).join('');
      };

      // Simple SHA-256 implementation (for demo purposes)
      const sha256 = (data: Uint8Array | string): Uint8Array => {
        // In a real implementation, this would use a proper SHA-256 library
        // For demo purposes, we'll create a deterministic hash
        const str = typeof data === 'string' ? data : Array.from(data).map(b => b.toString(16).padStart(2, '0')).join('');
        const hash = Array.from({length: 32}, (_, i) => {
          const charCode = str.charCodeAt(i % str.length);
          return (charCode + i * 7) % 256;
        });
        return new Uint8Array(hash);
      };

      // Helper function to create canonical transactions matching JSON schema
      const createTransaction = (_blockId: string, _txIndex: number, timestamp: number, nodeId: string, round: number, sequence: number): Transaction => {
        const txHash = Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join('');
        const from = Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join('');
        const to = Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join('');
        const amount = (Math.floor(Math.random() * 1000000) + 1000).toString();
        const signature = Array.from({length: 128}, () => Math.floor(Math.random() * 16).toString(16)).join('');
        
        // Create payload digest for transaction
        const payloadDigest = Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join('');
        
        // Create HashTimer with canonical v1 format
        const hashtimer = createHashTimer(timestamp, nodeId, round, sequence, 'Tx', payloadDigest);
        
        return {
          tx_hash: txHash,
          from,
          to,
          amount,
          fee: Math.floor(Math.random() * 1000) + 100,
          nonce: Math.floor(Math.random() * 10000),
          memo: Math.random() > 0.7 ? `Transaction ${_txIndex}` : undefined,
          signature,
          hashtimer
        };
      };
      
      // Generate genesis block
      const genesis: Block = {
        block_id: 'genesis',
        producer: {
          node_id: Array.from({length: 32}, () => Math.floor(Math.random() * 16).toString(16)).join(''),
          label: 'validator-1'
        },
        status: 'finalized',
        tx_count: 0,
        header_digest: Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join(''),
        hashtimer: createHashTimer(now - 300000, 'validator-1', currentRound, 0, 'Block'),
        parents: [], // Genesis block has no parents
        parent_rounds: [], // Genesis block has no parent rounds
        txs: [] // Genesis has no transactions
      };
      mockBlocks.push(genesis);

      // Generate blocks organized by rounds
      let blockId = 1;
      let sequence = 1;
      
      // Generate multiple rounds with blocks
      for (let roundOffset = 0; roundOffset < 10; roundOffset++) {
        const roundNumber = currentRound + roundOffset;
        const roundStartTime = now - 300000 + (roundOffset * roundDuration);
        const roundEndTime = roundStartTime + roundDuration;
        
        // Determine round state based on time
        let roundState: 'collecting' | 'aggregating' | 'finalized';
        if (roundEndTime < now - 1000) {
          roundState = 'finalized';
        } else if (roundEndTime < now) {
          roundState = 'aggregating';
        } else {
          roundState = 'collecting';
        }
        
        const roundBlocks: Block[] = [];
        
        // Generate 3-8 blocks per round
        const blocksInRound = Math.floor(Math.random() * 6) + 3;
        for (let i = 0; i < blocksInRound; i++) {
          const blockTimestamp = roundStartTime + (i * (roundDuration / blocksInRound));
          const validatorId = `validator-${(i % 3) + 1}`;
          const blockHashtimer = createHashTimer(blockTimestamp, validatorId, roundNumber, sequence, 'Block');
          
          // Generate transactions for this block
          const transactionCount = Math.floor(Math.random() * 50) + 10;
          const blockTransactions: Transaction[] = [];
          
          for (let txIndex = 0; txIndex < transactionCount; txIndex++) {
            const txTimestamp = blockTimestamp + (txIndex * 1000); // 1ms apart
            const transaction = createTransaction(`block-${blockId}`, txIndex, txTimestamp, validatorId, roundNumber, sequence + txIndex);
            blockTransactions.push(transaction);
          }
        
        // Generate parents for this block (1-3 parents, except for genesis)
        const parents: string[] = [];
        const parent_rounds: string[] = [];
        
        if (blockId > 1) { // Not genesis block
          const parentCount = Math.floor(Math.random() * 3) + 1; // 1-3 parents
          for (let p = 0; p < parentCount; p++) {
            // Generate parent hash (32-byte hex)
            const parentHash = Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join('');
            parents.push(parentHash);
            
            // Parent round is current round or previous round
            const parentRound = roundNumber - Math.floor(Math.random() * 2);
            parent_rounds.push(parentRound.toString());
          }
        }

        const block: Block = {
            block_id: `block-${blockId}`,
            producer: {
              node_id: Array.from({length: 32}, () => Math.floor(Math.random() * 16).toString(16)).join(''),
              label: validatorId
            },
            status: roundState === 'finalized' ? 'finalized' : 'pending',
            tx_count: transactionCount,
            header_digest: Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join(''),
            hashtimer: blockHashtimer,
            parents: parents,
            parent_rounds: parent_rounds,
            txs: blockTransactions
          };
          
          roundBlocks.push(block);
        mockBlocks.push(block);
        
          // Add connections
          if (i > 0) {
            mockConnections.push({ 
              from: `block-${blockId - 1}`, 
              to: block.block_id, 
              type: 'parent' 
            });
          }
          
          blockId++;
          sequence += transactionCount;
        }
        
        // Create round
        const round: Round = {
          version: 'v1',
          round_id: roundNumber.toString(),
          state: roundState === 'finalized' ? 'finalized' : roundState === 'aggregating' ? 'finalizing' : 'pending',
          time: {
            start_ns: (roundStartTime * 1_000_000).toString(),
            end_ns: (roundEndTime * 1_000_000).toString()
          },
          block_count: roundBlocks.length,
          zk_stark_proof: Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join(''),
          merkle_root: Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join(''),
          blocks: roundBlocks
        };
        
        mockRounds.push(round);
      }

      // Collect all transactions from all blocks, ordered by canonical HashTimer ordering
      const allTransactions = mockBlocks
        .flatMap(block => block.txs)
        .sort((a, b) => {
          // Primary: t_ns (nanoseconds)
          const tNsA = BigInt(a.hashtimer.time.t_ns);
          const tNsB = BigInt(b.hashtimer.time.t_ns);
          if (tNsA !== tNsB) return tNsA < tNsB ? -1 : 1;
          
          // Secondary: round
          const roundA = BigInt(a.hashtimer.position.round);
          const roundB = BigInt(b.hashtimer.position.round);
          if (roundA !== roundB) return roundA < roundB ? -1 : 1;
          
          // Tertiary: seq
          if (a.hashtimer.position.seq !== b.hashtimer.position.seq) {
            return a.hashtimer.position.seq - b.hashtimer.position.seq;
          }
          
          // Quaternary: node_id (lexicographic)
          if (a.hashtimer.node_id !== b.hashtimer.node_id) {
            return a.hashtimer.node_id < b.hashtimer.node_id ? -1 : 1;
          }
          
          // Quinary: payload_digest (lexicographic)
          return a.hashtimer.payload_digest < b.hashtimer.payload_digest ? -1 : 1;
        });

      setBlocks(mockBlocks);
      setRounds(mockRounds);
      setTransactions(allTransactions);
      setConnections(mockConnections);
    };

    generateMockData();
    
    if (autoRefresh) {
      const interval = setInterval(generateMockData, 5000); // Refresh every 5 seconds
      return () => clearInterval(interval);
    }
  }, [autoRefresh]);

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'finalized': return 'bg-green-500';
      case 'aggregating': return 'bg-blue-500';
      case 'collecting': return 'bg-yellow-500';
      case 'orphaned': return 'bg-red-500';
      default: return 'bg-gray-500';
    }
  };

  const getRoundStatusColor = (state: string) => {
    switch (state) {
      case 'finalized': return 'bg-green-100 border-green-500 text-green-800';
      case 'aggregating': return 'bg-blue-100 border-blue-500 text-blue-800';
      case 'collecting': return 'bg-yellow-100 border-yellow-500 text-yellow-800';
      default: return 'bg-gray-100 border-gray-500 text-gray-800';
    }
  };

  const formatTimestamp = (timestamp: number) => {
    const now = Date.now();
    const diff = now - timestamp;
    if (diff < 60000) return `${Math.floor(diff / 1000)}s ago`;
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    return `${Math.floor(diff / 3600000)}h ago`;
  };

  const renderDAGView = () => (
    <div className="relative min-h-[600px] bg-gray-50 dark:bg-gray-900 rounded-lg p-6 overflow-auto">
      <div className="text-sm text-gray-600 dark:text-gray-400 mb-4">
        DAG Visualization: Parallel blocks with HashTimer-based ordering
      </div>
      
      {/* Render blocks in layers */}
      {blocks.map((block, index) => {
        const layer = Math.floor(index / 3);
        const layerOffset = (index % 3) * 200;
        
        return (
          <div
            key={block.block_id}
            id={`block-${block.block_id}`}
            className={`absolute bg-blue-500 text-white rounded-lg p-3 cursor-pointer hover:scale-105 transition-transform shadow-lg`}
            style={{
              left: `${layer * 300 + layerOffset}px`,
              top: `${layer * 120}px`,
              width: '180px',
              zIndex: 10
            }}
            onClick={() => setSelectedBlock(block)}
          >
            <div className="text-xs font-mono truncate mb-1">{block.block_id}</div>
            <div className="text-xs opacity-90 truncate mb-1">{block.header_digest.substring(0, 8)}...</div>
            <div className="text-xs">Round: {block.hashtimer.position.round}</div>
            <div className="text-xs">Timer: {block.hashtimer.time.t_ns}ns</div>
            <div className="text-xs">Drift: {block.hashtimer.time.drift_ns}ns</div>
            <div className="text-xs">Txs: {block.tx_count}</div>
            <div className={`inline-block w-2 h-2 rounded-full ${getStatusColor(block.status)} mt-1`}></div>
          </div>
        );
      })}
      
      {/* Render connections */}
      <svg className="absolute inset-0 w-full h-full pointer-events-none" style={{ zIndex: 5 }}>
        {connections.map((conn, index) => {
          const fromBlock = blocks.find(b => b.block_id === conn.from);
          const toBlock = blocks.find(b => b.block_id === conn.to);
          
          if (!fromBlock || !toBlock) return null;
          
          const fromIndex = blocks.indexOf(fromBlock);
          const toIndex = blocks.indexOf(toBlock);
          const fromLayer = Math.floor(fromIndex / 3);
          const toLayer = Math.floor(toIndex / 3);
          const fromOffset = (fromIndex % 3) * 200;
          const toOffset = (toIndex % 3) * 200;
          
          const fromX = fromLayer * 300 + fromOffset + 90;
          const fromY = fromLayer * 120 + 60;
          const toX = toLayer * 300 + toOffset + 90;
          const toY = toLayer * 120 + 60;
          
          return (
            <line
              key={index}
              x1={fromX}
              y1={fromY}
              x2={toX}
              y2={toY}
              stroke={conn.type === 'parent' ? '#3b82f6' : '#10b981'}
              strokeWidth="2"
              markerEnd="url(#arrowhead)"
            />
          );
        })}
        
        {/* Arrow marker */}
        <defs>
          <marker
            id="arrowhead"
            markerWidth="10"
            markerHeight="7"
            refX="9"
            refY="3.5"
            orient="auto"
          >
            <polygon points="0 0, 10 3.5, 0 7" fill="#3b82f6" />
          </marker>
        </defs>
      </svg>
    </div>
  );

  const renderTimelineView = () => (
    <div className="space-y-4">
      {blocks
        .sort((a, b) => parseInt(b.hashtimer.time.t_ns) - parseInt(a.hashtimer.time.t_ns))
        .map((block) => (
          <div
            key={block.block_id}
            id={`block-${block.block_id}`}
            className={`border-l-4 border-blue-500 bg-white dark:bg-gray-800 p-4 rounded-r-lg shadow-sm hover:shadow-md transition-shadow cursor-pointer`}
            onClick={() => setSelectedBlock(block)}
          >
            <div className="flex justify-between items-start">
              <div>
                <h3 className="font-medium text-gray-900 dark:text-white">{block.block_id}</h3>
                <p className="text-sm text-gray-600 dark:text-gray-400 font-mono">
                  {block.header_digest.substring(0, 16)}...
                </p>
                <p className="text-xs text-gray-500">Round: {block.hashtimer.position.round}</p>
              </div>
              <div className="text-right">
                <div className="text-sm text-gray-500">{formatTimestamp(parseInt(block.hashtimer.time.t_ns) / 1_000_000)}</div>
                <div className={`inline-block px-2 py-1 rounded-full text-xs font-medium ${getStatusColor(block.status)} text-white`}>
                  {block.status}
                </div>
              </div>
            </div>
            <div className="mt-2 grid grid-cols-2 gap-4 text-sm">
              <div>
                <span className="text-gray-500">HashTimer (ns):</span>
                <span className="ml-1 font-medium font-mono">{block.hashtimer.time.t_ns}</span>
              </div>
              <div>
                <span className="text-gray-500">Drift:</span>
                <span className="ml-1 font-medium">{block.hashtimer.time.drift_ns}ns</span>
              </div>
              <div>
                <span className="text-gray-500">Precision:</span>
                <span className="ml-1 font-medium">{block.hashtimer.time.precision_ns}ns</span>
              </div>
              <div>
                <span className="text-gray-500">Sequence:</span>
                <span className="ml-1 font-medium">{block.hashtimer.position.seq}</span>
              </div>
              <div>
                <span className="text-gray-500">Txs:</span>
                <span className="ml-1 font-medium">{block.tx_count}</span>
              </div>
              <div>
                <span className="text-gray-500">Validator:</span>
                <span className="ml-1 font-medium">{block.producer.label}</span>
              </div>
            </div>
              <div className="mt-2 text-xs text-gray-500">
              <div>HashTimer Hash: {block.hashtimer.hash_timer_digest.substring(0, 16)}...</div>
              <div>Node ID: {block.hashtimer.node_id}</div>
            </div>
          </div>
        ))}
    </div>
  );

  const renderRoundsView = () => (
    <div className="space-y-6">
      {rounds
        .sort((a, b) => parseInt(b.round_id) - parseInt(a.round_id))
        .map((round) => (
          <div
            key={round.round_id}
            className={`border-2 rounded-lg p-4 ${getRoundStatusColor(round.state)} cursor-pointer hover:shadow-md transition-shadow`}
            onClick={() => setSelectedRound(round)}
          >
            <div className="flex justify-between items-start mb-4">
              <div>
                <h3 className="text-lg font-semibold">Round {round.round_id}</h3>
                <p className="text-sm opacity-75">
                  {formatTimestamp(parseInt(round.time.start_ns) / 1_000_000)} - {formatTimestamp(parseInt(round.time.end_ns) / 1_000_000)}
                </p>
              </div>
              <div className="text-right">
                <div className={`inline-block px-3 py-1 rounded-full text-sm font-medium ${getRoundStatusColor(round.state)}`}>
                  {round.state.toUpperCase()}
                </div>
                <div className="text-sm mt-1">
                  {round.blocks.length} blocks
                </div>
              </div>
            </div>
            
            <div className="mb-3 p-2 bg-white bg-opacity-50 rounded text-xs">
              <div className="font-medium">ZK Proof:</div>
              <div className="font-mono">{round.zk_stark_proof.substring(0, 32)}...</div>
            </div>
            
            {round.merkle_root && (
              <div className="mb-3 p-2 bg-white bg-opacity-50 rounded text-xs">
                <div className="font-medium">Merkle Root:</div>
                <div className="font-mono">{round.merkle_root.substring(0, 32)}...</div>
              </div>
            )}
            
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {round.blocks.map((block) => (
                <div
                  key={block.block_id}
                  id={`block-${block.block_id}`}
                  className="bg-white bg-opacity-70 rounded p-3 text-sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    setSelectedBlock(block);
                  }}
                >
                  <div className="font-medium">{block.block_id}</div>
                  <div className="text-xs opacity-75">
                    HashTimer: {block.hashtimer.time.t_ns}ns
                  </div>
                  <div className="text-xs opacity-75">
                    Drift: {block.hashtimer.time.drift_ns}ns | Precision: {block.hashtimer.time.precision_ns}ns
                  </div>
                  <div className="text-xs opacity-75">
                    {block.tx_count} txs | {block.producer.label}
                  </div>
                  <div className="text-xs opacity-75">
                    Parents ({block.parents.length}): {block.parents.length > 0 ? 
                      block.parents.map(p => p.substring(0, 4)).join(' ') : 'Genesis'
                    }
                  </div>
                  <div className={`inline-block w-2 h-2 rounded-full ${getStatusColor(block.status)} mt-1`}></div>
                </div>
              ))}
            </div>
          </div>
        ))}
    </div>
  );

  const renderTransactionsView = () => (
    <div className="space-y-4">
      <div className="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-4 mb-4">
        <h3 className="font-medium text-blue-900 dark:text-blue-100 mb-2">Transactions Ordered by HashTimer</h3>
        <p className="text-sm text-blue-800 dark:text-blue-200">
          All transactions across all blocks, sorted by their HashTimer v1 timestamps (nanosecond precision).
          This is the typical IPPAN ordering mechanism for deterministic transaction processing.
        </p>
      </div>
      
      {transactions.map((tx, index) => (
        <div
          key={tx.tx_hash}
          className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 hover:shadow-md transition-shadow cursor-pointer"
          onClick={() => setSelectedTransaction(tx)}
        >
          <div className="flex justify-between items-start mb-3">
            <div>
              <h3 className="font-medium text-gray-900 dark:text-white">
                Transaction #{index + 1}
              </h3>
              <p className="text-sm text-gray-600 dark:text-gray-400 font-mono">
                {tx.tx_hash.substring(0, 16)}...
              </p>
            </div>
            <div className="text-right">
              <div className="text-sm text-gray-500">
                Fee: {tx.fee} | Nonce: {tx.nonce}
              </div>
              <div className="text-xs text-gray-400">
                Round: {tx.hashtimer.position.round} | Seq: {tx.hashtimer.position.seq}
              </div>
            </div>
          </div>
          
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
            <div>
              <span className="text-gray-500">HashTimer (ns):</span>
              <span className="ml-1 font-medium font-mono">{tx.hashtimer.time.t_ns}</span>
            </div>
            <div>
              <span className="text-gray-500">Drift:</span>
              <span className="ml-1 font-medium">{tx.hashtimer.time.drift_ns}ns</span>
            </div>
            <div>
              <span className="text-gray-500">Precision:</span>
              <span className="ml-1 font-medium">{tx.hashtimer.time.precision_ns}ns</span>
            </div>
            <div>
              <span className="text-gray-500">Node ID:</span>
              <span className="ml-1 font-medium">{tx.hashtimer.node_id}</span>
            </div>
          </div>
          
          <div className="mt-3 text-xs text-gray-500">
            <div>HashTimer v1 (32-byte digest): {tx.hashtimer.hash_timer_digest.substring(0, 16)}...</div>
            <div className="mt-1">
              <span className="text-blue-600">Time:</span> {tx.hashtimer.time.t_ns}ns | 
              <span className="text-orange-600"> Kind:</span> {tx.hashtimer.position.kind}
            </div>
            <div className="mt-1">
              <span className="text-gray-600">Node ID:</span> {tx.hashtimer.node_id.substring(0, 16)}...
            </div>
            <div>
              From: {tx.from.substring(0, 16)}... | To: {tx.to.substring(0, 16)}... | Amount: {tx.amount}
            </div>
          </div>
        </div>
      ))}
    </div>
  );

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-gray-900 dark:text-white">Live Blocks</h1>
        <p className="mt-2 text-gray-600 dark:text-gray-400">
          Real-time IPPAN blockchain visualization with Round-based finalization and nanosecond-precision HashTimers
        </p>
      </div>

      {/* Controls */}
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-4">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div className="flex items-center space-x-4">
            <label className="text-sm font-medium text-gray-700 dark:text-gray-300">View Mode:</label>
            <select
              value={viewMode}
              onChange={(e) => setViewMode(e.target.value as any)}
              className="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-white"
            >
              <option value="rounds">Rounds View</option>
              <option value="transactions">Transactions (HashTimer Ordered)</option>
              <option value="dag">DAG Visualization</option>
              <option value="timeline">Timeline</option>
            </select>
          </div>
          
          <div className="flex items-center space-x-4">
            <label className="flex items-center">
              <input
                type="checkbox"
                checked={autoRefresh}
                onChange={(e) => setAutoRefresh(e.target.checked)}
                className="mr-2"
              />
              <span className="text-sm text-gray-700 dark:text-gray-300">Auto-refresh</span>
            </label>
            
            <button
              onClick={() => window.location.reload()}
              className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              Refresh Now
            </button>
          </div>
        </div>
      </div>

      {/* IPPAN Info */}
      <div className="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-4">
        <h3 className="font-medium text-blue-900 dark:text-blue-100 mb-2">IPPAN Blockchain Features</h3>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4 text-sm text-blue-800 dark:text-blue-200">
          <div>
            <strong>Round-based Finalization:</strong> Blocks finalize all together in 200ms rounds
          </div>
          <div>
            <strong>HashTimer v1:</strong> 96-byte input → 32-byte SHA-256 digest for deterministic ordering
          </div>
          <div>
            <strong>ZK-STARK Proofs:</strong> Succinct proofs for round aggregation
          </div>
          <div>
            <strong>DAG Structure:</strong> Parallel block creation with HashTimer-based ordering
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow">
        {viewMode === 'rounds' && renderRoundsView()}
        {viewMode === 'transactions' && renderTransactionsView()}
        {viewMode === 'dag' && renderDAGView()}
        {viewMode === 'timeline' && renderTimelineView()}
      </div>

      {/* Block Details Modal */}
      {selectedBlock && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-2xl w-full mx-4 max-h-[80vh] overflow-y-auto">
            <div className="flex justify-between items-start mb-4">
              <h2 className="text-xl font-bold text-gray-900 dark:text-white">Block Details</h2>
              <button
                onClick={() => setSelectedBlock(null)}
                className="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
              >
                ✕
              </button>
            </div>
            
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Block ID</label>
                <p className="text-sm text-gray-900 dark:text-white font-mono">{selectedBlock.block_id}</p>
              </div>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Hash</label>
                <p className="text-sm text-gray-900 dark:text-white font-mono break-all">{selectedBlock.header_digest}</p>
              </div>
              
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Timestamp</label>
                  <p className="text-sm text-gray-900 dark:text-white">{new Date(parseInt(selectedBlock.hashtimer.time.t_ns) / 1_000_000).toLocaleString()}</p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">HashTimer (ns)</label>
                  <p className="text-sm text-gray-900 dark:text-white font-mono">{selectedBlock.hashtimer.time.t_ns}</p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Drift (ns)</label>
                  <p className="text-sm text-gray-900 dark:text-white">{selectedBlock.hashtimer.time.drift_ns}</p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Precision (ns)</label>
                  <p className="text-sm text-gray-900 dark:text-white">{selectedBlock.hashtimer.time.precision_ns}</p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Sequence</label>
                  <p className="text-sm text-gray-900 dark:text-white">{selectedBlock.hashtimer.position.seq}</p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Round</label>
                  <p className="text-sm text-gray-900 dark:text-white">{selectedBlock.hashtimer.position.round}</p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Status</label>
                  <p className="text-sm text-gray-900 dark:text-white capitalize">{selectedBlock.status}</p>
                </div>
              </div>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Validator</label>
                <p className="text-sm text-gray-900 dark:text-white">{selectedBlock.producer.label} ({selectedBlock.producer.node_id})</p>
              </div>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Parents ({selectedBlock.parents.length})</label>
                {selectedBlock.parents.length > 0 ? (
                  <div className="space-y-2">
                    {selectedBlock.parents.map((parent, index) => (
                      <div key={index} className="flex items-center space-x-2">
                        <span className="text-xs text-gray-500 dark:text-gray-400">Round {selectedBlock.parent_rounds[index]}:</span>
                        <span className="text-sm text-gray-900 dark:text-white font-mono bg-gray-100 dark:bg-gray-700 px-2 py-1 rounded">
                          {parent.substring(0, 8)}...{parent.substring(56)}
                        </span>
                        <button 
                          onClick={() => navigator.clipboard.writeText(parent)}
                          className="text-xs text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-200"
                          title="Copy full hash"
                        >
                          📋
                        </button>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-gray-500 dark:text-gray-400 italic">Genesis block (no parents)</p>
                )}
              </div>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Transaction Count</label>
                <p className="text-sm text-gray-900 dark:text-white">{selectedBlock.tx_count}</p>
              </div>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">State Root</label>
                <p className="text-sm text-gray-900 dark:text-white font-mono break-all">{selectedBlock.header_digest}</p>
              </div>
              
            </div>
          </div>
        </div>
      )}

      {/* Round Details Modal */}
      {selectedRound && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-4xl w-full mx-4 max-h-[80vh] overflow-y-auto">
            <div className="flex justify-between items-start mb-4">
              <h2 className="text-xl font-bold text-gray-900 dark:text-white">Round {selectedRound.round_id} Details</h2>
              <button
                onClick={() => setSelectedRound(null)}
                className="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
              >
                ✕
              </button>
            </div>
            
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Start Time</label>
                  <p className="text-sm text-gray-900 dark:text-white">{new Date(parseInt(selectedRound.time.start_ns) / 1_000_000).toLocaleString()}</p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">End Time</label>
                  <p className="text-sm text-gray-900 dark:text-white">
                    {new Date(parseInt(selectedRound.time.end_ns) / 1_000_000).toLocaleString()}
                  </p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">State</label>
                  <p className="text-sm text-gray-900 dark:text-white capitalize">{selectedRound.state}</p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Block Count</label>
                  <p className="text-sm text-gray-900 dark:text-white">{selectedRound.block_count}</p>
                </div>
              </div>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">ZK-STARK Proof</label>
                <p className="text-sm text-gray-900 dark:text-white font-mono break-all">{selectedRound.zk_stark_proof}</p>
              </div>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Merkle Root</label>
                <p className="text-sm text-gray-900 dark:text-white font-mono break-all">{selectedRound.merkle_root}</p>
              </div>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Blocks in Round</label>
                <div className="space-y-2 max-h-60 overflow-y-auto">
                  {selectedRound.blocks.map((block) => (
                    <div
                      key={block.block_id}
                      className="border rounded p-3 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700"
                      onClick={() => {
                        setSelectedBlock(block);
                        setSelectedRound(null);
                      }}
                    >
                      <div className="flex justify-between items-start">
                        <div>
                          <div className="font-medium">{block.block_id}</div>
                          <div className="text-sm text-gray-600 dark:text-gray-400">
                            HashTimer: {block.hashtimer.time.t_ns}ns | Drift: {block.hashtimer.time.drift_ns}ns
                          </div>
                          <div className="text-sm text-gray-600 dark:text-gray-400">
                            {block.tx_count} txs | {block.producer.label}
                          </div>
                        </div>
                        <div className={`inline-block px-2 py-1 rounded-full text-xs font-medium ${getStatusColor(block.status)} text-white`}>
                          {block.status}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Transaction Details Modal */}
      {selectedTransaction && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-2xl w-full mx-4 max-h-[80vh] overflow-y-auto">
            <div className="flex justify-between items-start mb-4">
              <h2 className="text-xl font-bold text-gray-900 dark:text-white">Transaction Details</h2>
              <button
                onClick={() => setSelectedTransaction(null)}
                className="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
              >
                ✕
              </button>
            </div>
            
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Transaction Hash</label>
                <p className="text-sm text-gray-900 dark:text-white font-mono break-all">{selectedTransaction.tx_hash}</p>
              </div>
              
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">From</label>
                  <p className="text-sm text-gray-900 dark:text-white font-mono">{selectedTransaction.from}</p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">To</label>
                  <p className="text-sm text-gray-900 dark:text-white font-mono">{selectedTransaction.to}</p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Amount</label>
                  <p className="text-sm text-gray-900 dark:text-white">{selectedTransaction.amount}</p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Fee</label>
                  <p className="text-sm text-gray-900 dark:text-white">{selectedTransaction.fee}</p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Nonce</label>
                  <p className="text-sm text-gray-900 dark:text-white">{selectedTransaction.nonce}</p>
                </div>
                {selectedTransaction.memo && (
                  <div>
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Memo</label>
                    <p className="text-sm text-gray-900 dark:text-white">{selectedTransaction.memo}</p>
                  </div>
                )}
              </div>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">HashTimer Details (v1)</label>
                <div className="bg-gray-50 dark:bg-gray-700 rounded p-3">
                  <div className="font-mono text-xs break-all bg-gray-100 dark:bg-gray-600 p-2 rounded">
                    <div><span className="text-blue-600">Time (ns):</span> {selectedTransaction.hashtimer.time.t_ns}</div>
                    <div><span className="text-green-600">Precision:</span> {selectedTransaction.hashtimer.time.precision_ns}ns | <span className="text-orange-600">Drift:</span> {selectedTransaction.hashtimer.time.drift_ns}ns</div>
                    <div><span className="text-purple-600">Round:</span> {selectedTransaction.hashtimer.position.round} | <span className="text-indigo-600">Seq:</span> {selectedTransaction.hashtimer.position.seq} | <span className="text-red-600">Kind:</span> {selectedTransaction.hashtimer.position.kind}</div>
                    <div><span className="text-gray-600">Node ID:</span> {selectedTransaction.hashtimer.node_id}</div>
                    <div><span className="text-yellow-600">Payload Digest:</span> {selectedTransaction.hashtimer.payload_digest.substring(0, 32)}...</div>
                    <div><span className="text-cyan-600">HashTimer Digest:</span> {selectedTransaction.hashtimer.hash_timer_digest}</div>
                  </div>
                </div>
              </div>
              
              
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Signature</label>
                <p className="text-sm text-gray-900 dark:text-white font-mono break-all">{selectedTransaction.signature}</p>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
