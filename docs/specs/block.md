# IPPAN Block Spec (v1)

## Size
- **Hard maximum**: **32 KB** (32,768 bytes)
- **Typical range**: 4–32 KB
- Blocks store **transaction hashes only** (no inlined payloads).

## Header (~184 B)
- `block_hash` (32 B)
- `parent_hash` (32 B)
- `round` (8 B)
- `timestamp_ns` (8 B)
- `validator_id` (32 B)
- `block_size_bytes` (4 B)
- `tx_count` (4 B)
- `merkle_root` (32 B)
- `hashtimer_digest` (32 B)

## Proofs (Per-Round)
- ZK-STARK proof is generated **once per round** (not per block).
- Typical size: **50–100+ KB** per round.

## Performance Targets
- Block cadence: **10–50 ms**
- Round finality: **100–250 ms**
- Target throughput: **1–10 million TPS**

## Config
- **Soft target size** (default 24 KB), env: `IPPAN_BLOCK_SOFT_TARGET_KB` (clamped to ≤ 32 KB).
- The 32 KB hard cap is enforced in code and cannot be exceeded.

## Block Structure

### BlockHeader
```rust
pub struct BlockHeader {
    pub hash: BlockHash,                    // 32 bytes
    pub round: u64,                         // 8 bytes
    pub height: u64,                        // 8 bytes
    pub validator_id: NodeId,               // 32 bytes
    pub hashtimer: HashTimer,               // 32 bytes
    pub parent_hashes: Vec<BlockHash>,      // Variable (1-8 parents)
    pub parent_rounds: Vec<u64>,            // Variable (1-8 rounds)
    pub timestamp_ns: u64,                  // 8 bytes
    pub block_size_bytes: u32,              // 4 bytes
    pub tx_count: u32,                      // 4 bytes
    pub merkle_root: [u8; 32],              // 32 bytes
}
```

### Block
```rust
pub struct Block {
    pub header: BlockHeader,                // ~184 bytes + variable parent data
    pub tx_hashes: Vec<TransactionHash>,    // 32 bytes per transaction hash
    pub signature: Option<[u8; 64]>,        // Optional 64-byte signature
}
```

## Size Calculation

The block size is calculated as:
- Header size: ~184 bytes + (parent_hashes.len() * 32) + (parent_rounds.len() * 8)
- Transaction hashes: tx_hashes.len() * 32 bytes
- Optional signature: 64 bytes (if present)

## Validation Rules

1. **Size Limit**: Total block size must not exceed 32,768 bytes
2. **Parent Validation**: 1-8 parent blocks (except genesis)
3. **Merkle Root**: Must match calculated root from transaction hashes
4. **Hash Validation**: Block hash must match calculated hash
5. **HashTimer**: Must be valid and within acceptable time bounds

## Error Handling

Blocks that exceed the size limit will return a `BlockError::TooLarge` error with details about the actual size vs. maximum allowed size.

## Environment Configuration

- `IPPAN_BLOCK_SOFT_TARGET_KB`: Set soft target size in KB (clamped to 4-32 KB range)
- Default soft target: 24 KB
- Hard limit: 32 KB (cannot be exceeded)

## Implementation Notes

- Blocks are created with `Block::new()` which enforces size limits
- Size estimation is performed before block creation
- Merkle root is calculated from transaction hashes
- ZK-STARK proofs are generated per round, not per block
- Transaction payloads are stored separately from blocks
