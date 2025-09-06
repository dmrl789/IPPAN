# 🛠️ IPPAN Schema Implementation Guide

## Overview

This guide provides step-by-step instructions for implementing the canonical IPPAN JSON schema in your applications, including TypeScript/JavaScript, Rust, and other languages.

## Quick Start

### 1. TypeScript/JavaScript Implementation

#### Install Dependencies

```bash
npm install ajv ajv-formats
```

#### Schema Validation

```typescript
import Ajv from 'ajv';
import addFormats from 'ajv-formats';

// Load the IPPAN schema
const ippanRoundSchema = {
  "$id": "https://ippan.org/schema/round-v1.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  // ... complete schema from JSON_SCHEMA_SPECIFICATION.md
};

const ajv = new Ajv();
addFormats(ajv);

const validate = ajv.compile(ippanRoundSchema);

// Validate IPPAN data
function validateIppanRound(data: any): boolean {
  const isValid = validate(data);
  if (!isValid) {
    console.error('Validation errors:', validate.errors);
  }
  return isValid;
}
```

#### TypeScript Types

```typescript
// HashTimer v1 Types
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

// Transaction Types
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

// Block Types
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
  txs: Transaction[];          // Array of transactions
}

// Round Types
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
```

#### Canonical Ordering Implementation

```typescript
// Sort transactions by canonical ordering
function sortTransactionsByHashTimer(transactions: Transaction[]): Transaction[] {
  return transactions.sort((a, b) => {
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
}
```

### 2. Rust Implementation

#### Cargo.toml Dependencies

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
schemars = "0.8"
jsonschema = "0.16"
```

#### Rust Types

```rust
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Tx,
    Block,
    Round,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HashTimerTime {
    pub t_ns: String,           // Nanoseconds since epoch (stringified big int)
    pub precision_ns: u32,      // Precision quantum
    pub drift_ns: String,       // Clock drift (stringified signed int)
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HashTimerPosition {
    pub round: String,          // Consensus round (stringified big int)
    pub seq: u32,               // Sequence number within round
    pub kind: EventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HashTimer {
    pub version: String,        // Always "v1"
    pub time: HashTimerTime,
    pub position: HashTimerPosition,
    pub node_id: String,        // 16-byte node identifier (32 hex chars)
    pub payload_digest: String, // SHA-256 of event payload (64 hex chars)
    pub hash_timer_digest: String, // SHA-256 of 96-byte HashTimer buffer (64 hex chars)
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Transaction {
    pub tx_hash: String,        // Transaction hash (64 hex chars)
    pub from: String,           // Sender address (32 hex chars for flexibility)
    pub to: String,             // Recipient address (32 hex chars for flexibility)
    pub amount: String,         // Amount (stringified big int)
    pub fee: u64,               // Transaction fee
    pub nonce: u64,             // Transaction nonce
    pub memo: Option<String>,   // Optional memo
    pub signature: String,      // Transaction signature
    pub hashtimer: HashTimer,   // Canonical HashTimer v1
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BlockProducer {
    pub node_id: String,        // 16-byte node identifier (32 hex chars)
    pub label: String,          // Human-readable label (e.g., "validator-1")
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BlockStatus {
    Pending,
    Finalized,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Block {
    pub block_id: String,       // Block identifier
    pub producer: BlockProducer,
    pub status: BlockStatus,
    pub tx_count: u32,          // Transaction count
    pub header_digest: String,  // Block header digest (64 hex chars)
    pub hashtimer: HashTimer,   // Canonical HashTimer v1
    pub txs: Vec<Transaction>,  // Array of transactions
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RoundTime {
    pub start_ns: String,       // Start time in nanoseconds (stringified big int)
    pub end_ns: String,         // End time in nanoseconds (stringified big int)
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoundState {
    Pending,
    Finalizing,
    Finalized,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Round {
    pub version: String,        // Always "v1"
    pub round_id: String,       // Round identifier (stringified big int)
    pub state: RoundState,
    pub time: RoundTime,
    pub block_count: u32,       // Number of blocks in this round
    pub zk_stark_proof: String, // ZK-STARK proof (64 hex chars)
    pub merkle_root: String,    // Merkle root (64 hex chars)
    pub blocks: Vec<Block>,     // Array of blocks
}
```

#### Schema Generation

```rust
use schemars::schema_for;

// Generate JSON schema from Rust types
fn generate_schema() -> serde_json::Value {
    let schema = schema_for!(Round);
    serde_json::to_value(schema).unwrap()
}
```

#### Canonical Ordering Implementation

```rust
use std::cmp::Ordering;

impl Transaction {
    pub fn compare_canonical(&self, other: &Transaction) -> Ordering {
        // Primary: t_ns (nanoseconds)
        let t_ns_cmp = self.hashtimer.time.t_ns.cmp(&other.hashtimer.time.t_ns);
        if t_ns_cmp != Ordering::Equal {
            return t_ns_cmp;
        }
        
        // Secondary: round
        let round_cmp = self.hashtimer.position.round.cmp(&other.hashtimer.position.round);
        if round_cmp != Ordering::Equal {
            return round_cmp;
        }
        
        // Tertiary: seq
        let seq_cmp = self.hashtimer.position.seq.cmp(&other.hashtimer.position.seq);
        if seq_cmp != Ordering::Equal {
            return seq_cmp;
        }
        
        // Quaternary: node_id (lexicographic)
        let node_cmp = self.hashtimer.node_id.cmp(&other.hashtimer.node_id);
        if node_cmp != Ordering::Equal {
            return node_cmp;
        }
        
        // Quinary: payload_digest (lexicographic)
        self.hashtimer.payload_digest.cmp(&other.hashtimer.payload_digest)
    }
}

// Sort transactions by canonical ordering
pub fn sort_transactions_canonical(transactions: &mut Vec<Transaction>) {
    transactions.sort_by(|a, b| a.compare_canonical(b));
}
```

### 3. Python Implementation

#### Install Dependencies

```bash
pip install jsonschema pydantic
```

#### Python Types with Pydantic

```python
from pydantic import BaseModel, Field
from typing import Optional, List, Literal
from enum import Enum

class EventKind(str, Enum):
    TX = "Tx"
    BLOCK = "Block"
    ROUND = "Round"

class HashTimerTime(BaseModel):
    t_ns: str = Field(..., description="Nanoseconds since epoch (stringified big int)")
    precision_ns: int = Field(..., ge=1, description="Precision quantum")
    drift_ns: str = Field(..., description="Clock drift (stringified signed int)")

class HashTimerPosition(BaseModel):
    round: str = Field(..., description="Consensus round (stringified big int)")
    seq: int = Field(..., ge=1, description="Sequence number within round")
    kind: EventKind

class HashTimer(BaseModel):
    version: Literal["v1"] = "v1"
    time: HashTimerTime
    position: HashTimerPosition
    node_id: str = Field(..., regex=r"^[0-9a-fA-F]{32}$", description="16-byte node identifier")
    payload_digest: str = Field(..., regex=r"^[0-9a-fA-F]{64}$", description="SHA-256 of event payload")
    hash_timer_digest: str = Field(..., regex=r"^[0-9a-fA-F]{64}$", description="SHA-256 of 96-byte HashTimer buffer")

class Transaction(BaseModel):
    tx_hash: str = Field(..., regex=r"^[0-9a-fA-F]{64}$", description="Transaction hash")
    from_: str = Field(..., alias="from", regex=r"^[0-9a-fA-F]{64}$", description="Sender address")
    to: str = Field(..., regex=r"^[0-9a-fA-F]{64}$", description="Recipient address")
    amount: str = Field(..., regex=r"^[0-9]+$", description="Amount (stringified big int)")
    fee: int = Field(..., ge=0, description="Transaction fee")
    nonce: int = Field(..., ge=0, description="Transaction nonce")
    memo: Optional[str] = Field(None, description="Optional memo")
    signature: str = Field(..., regex=r"^[0-9a-fA-F]+$", description="Transaction signature")
    hashtimer: HashTimer

class BlockProducer(BaseModel):
    node_id: str = Field(..., regex=r"^[0-9a-fA-F]{32}$", description="16-byte node identifier")
    label: str = Field(..., description="Human-readable label")

class BlockStatus(str, Enum):
    PENDING = "pending"
    FINALIZED = "finalized"

class Block(BaseModel):
    block_id: str = Field(..., description="Block identifier")
    producer: BlockProducer
    status: BlockStatus
    tx_count: int = Field(..., ge=0, description="Transaction count")
    header_digest: str = Field(..., regex=r"^[0-9a-fA-F]{64}$", description="Block header digest")
    hashtimer: HashTimer
    txs: List[Transaction]

class RoundTime(BaseModel):
    start_ns: str = Field(..., regex=r"^[0-9]+$", description="Start time in nanoseconds")
    end_ns: str = Field(..., regex=r"^[0-9]+$", description="End time in nanoseconds")

class RoundState(str, Enum):
    PENDING = "pending"
    FINALIZING = "finalizing"
    FINALIZED = "finalized"
    REJECTED = "rejected"

class Round(BaseModel):
    version: Literal["v1"] = "v1"
    round_id: str = Field(..., regex=r"^[0-9]+$", description="Round identifier")
    state: RoundState
    time: RoundTime
    block_count: int = Field(..., ge=0, description="Number of blocks in this round")
    zk_stark_proof: str = Field(..., regex=r"^[0-9a-fA-F]{64}$", description="ZK-STARK proof")
    merkle_root: str = Field(..., regex=r"^[0-9a-fA-F]{64}$", description="Merkle root")
    blocks: List[Block]
```

#### Canonical Ordering Implementation

```python
from typing import List

def sort_transactions_canonical(transactions: List[Transaction]) -> List[Transaction]:
    """Sort transactions by canonical ordering: (t_ns, round, seq, node_id, payload_digest)"""
    
    def compare_transactions(a: Transaction, b: Transaction) -> int:
        # Primary: t_ns (nanoseconds)
        t_ns_cmp = int(a.hashtimer.time.t_ns) - int(b.hashtimer.time.t_ns)
        if t_ns_cmp != 0:
            return t_ns_cmp
        
        # Secondary: round
        round_cmp = int(a.hashtimer.position.round) - int(b.hashtimer.position.round)
        if round_cmp != 0:
            return round_cmp
        
        # Tertiary: seq
        seq_cmp = a.hashtimer.position.seq - b.hashtimer.position.seq
        if seq_cmp != 0:
            return seq_cmp
        
        # Quaternary: node_id (lexicographic)
        node_cmp = (a.hashtimer.node_id > b.hashtimer.node_id) - (a.hashtimer.node_id < b.hashtimer.node_id)
        if node_cmp != 0:
            return node_cmp
        
        # Quinary: payload_digest (lexicographic)
        return (a.hashtimer.payload_digest > b.hashtimer.payload_digest) - (a.hashtimer.payload_digest < b.hashtimer.payload_digest)
    
    return sorted(transactions, key=lambda x: (
        int(x.hashtimer.time.t_ns),
        int(x.hashtimer.position.round),
        x.hashtimer.position.seq,
        x.hashtimer.node_id,
        x.hashtimer.payload_digest
    ))
```

## Best Practices

### 1. Big Integer Handling

Always use stringified big integers for large numbers to avoid precision loss:

```typescript
// ✅ Correct
const amount = "12345678901234567890";

// ❌ Incorrect - may lose precision
const amount = 12345678901234567890;
```

### 2. HashTimer Validation

Always validate HashTimer structure before processing:

```typescript
function validateHashTimer(hashtimer: HashTimer): boolean {
  // Check version
  if (hashtimer.version !== 'v1') return false;
  
  // Check time fields
  if (!/^\d+$/.test(hashtimer.time.t_ns)) return false;
  if (!/^-?\d+$/.test(hashtimer.time.drift_ns)) return false;
  
  // Check position fields
  if (!/^\d+$/.test(hashtimer.position.round)) return false;
  if (!['Tx', 'Block', 'Round'].includes(hashtimer.position.kind)) return false;
  
  // Check hex fields
  if (!/^[0-9a-fA-F]{32}$/.test(hashtimer.node_id)) return false;
  if (!/^[0-9a-fA-F]{64}$/.test(hashtimer.payload_digest)) return false;
  if (!/^[0-9a-fA-F]{64}$/.test(hashtimer.hash_timer_digest)) return false;
  
  return true;
}
```

### 3. Canonical Ordering

Always use canonical ordering for deterministic processing:

```typescript
// Sort transactions before processing
const sortedTransactions = sortTransactionsByHashTimer(transactions);

// Process in canonical order
for (const tx of sortedTransactions) {
  processTransaction(tx);
}
```

### 4. Error Handling

Implement proper error handling for schema validation:

```typescript
function processIppanData(data: any): Round | null {
  try {
    // Validate against schema
    if (!validateIppanRound(data)) {
      console.error('Schema validation failed:', validate.errors);
      return null;
    }
    
    // Parse and return
    return data as Round;
  } catch (error) {
    console.error('Error processing IPPAN data:', error);
    return null;
  }
}
```

## Testing

### Unit Tests

```typescript
import { validateIppanRound, sortTransactionsByHashTimer } from './ippan-schema';

describe('IPPAN Schema', () => {
  test('validates correct round data', () => {
    const validRound = {
      version: 'v1',
      round_id: '1234567890',
      state: 'finalized',
      time: {
        start_ns: '1756995008000000000',
        end_ns: '1756995008250000000'
      },
      block_count: 1,
      zk_stark_proof: 'a'.repeat(64),
      merkle_root: 'b'.repeat(64),
      blocks: []
    };
    
    expect(validateIppanRound(validRound)).toBe(true);
  });
  
  test('rejects invalid round data', () => {
    const invalidRound = {
      version: 'v2', // Invalid version
      round_id: '1234567890',
      // Missing required fields
    };
    
    expect(validateIppanRound(invalidRound)).toBe(false);
  });
  
  test('sorts transactions canonically', () => {
    const transactions = [
      createTransaction('2000000000', '2', 2),
      createTransaction('1000000000', '1', 1),
      createTransaction('1000000000', '1', 2),
    ];
    
    const sorted = sortTransactionsByHashTimer(transactions);
    
    expect(sorted[0].hashtimer.time.t_ns).toBe('1000000000');
    expect(sorted[0].hashtimer.position.seq).toBe(1);
    expect(sorted[1].hashtimer.position.seq).toBe(2);
    expect(sorted[2].hashtimer.time.t_ns).toBe('2000000000');
  });
});
```

## Migration Guide

### From Legacy Format

If you're migrating from a legacy format, follow these steps:

1. **Update data structures** to match the canonical schema
2. **Convert big integers** to strings
3. **Implement canonical ordering** for transactions
4. **Update validation** to use the new schema
5. **Test thoroughly** with real IPPAN data

### Version Compatibility

- **Current Version**: v1
- **Backward Compatibility**: Maintained through version tags
- **Forward Compatibility**: New fields will be optional in future versions

## Resources

- [Complete JSON Schema Specification](./JSON_SCHEMA_SPECIFICATION.md)
- [IPPAN Product Requirements Document](./IPPAN_PRD.md)
- [API Documentation](./API_DOCUMENTATION.md)
- [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12/schema)
