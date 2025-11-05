# IPPAN — HashTimer Hierarchy Instructions for Cursor Agents

## Overview
IPPAN uses **three deterministic HashTimer layers** for temporal ordering:
1. **Transaction HashTimer** — microsecond level
2. **Block HashTimer** — 10–50 ms level
3. **Round HashTimer** — 100–250 ms level

Each layer *derives deterministically* from the previous one (Tx → Block → Round).

## Cursor Instructions

### Core Principles
- Always **prefix structures** explicitly:  
  - `TxHashTimer`, `BlockHashTimer`, `RoundHashTimer`
- Never reuse one type for another.
- Maintain **unidirectional derivation**:
  - Transactions produce BlockHashTimers  
  - Blocks produce RoundHashTimers  
  - Rounds never modify Block or Tx timers.

### Validation Rules
- `TxHashTimer` ∈ `[Block.start, Block.end]`
- `BlockHashTimer` ∈ `[Round.start, Round.end]`
- Each HashTimer must be verifiable via:

```rust
fn verify_hash_timer(source: &[u8], reference: &Hash) -> bool
```

*Round finality depends on **median(BlockHashTimers)**, not external clocks.*

### Implementation Guidelines

1. In `crates/time/src/lib.rs`, define:

    ```rust
    pub struct TxHashTimer(pub [u8; 32]);
    pub struct BlockHashTimer(pub [u8; 32]);
    pub struct RoundHashTimer(pub [u8; 32]);
    ```

    Implement `Serialize`, `Deserialize`, and `Display`.

2. In `crates/core/src/block.rs`, ensure:

    ```rust
    block.hash_timer: BlockHashTimer
    block.tx_hash_timers: Vec<TxHashTimer>
    ```

3. In `crates/consensus/src/round.rs`:

    ```rust
    round.hash_timer: RoundHashTimer
    round.blocks: Vec<BlockHashTimer>
    ```

4. Enforce deterministic derivation:

    ```rust
    BlockHashTimer = hash(median(TxHashTimers) || prev_block_hash || validator_pubkey)
    RoundHashTimer = hash(median(BlockHashTimers) || validator_commitments || network_time_window)
    ```

5. Anywhere you reference "hash timer", make sure Cursor chooses the correct layer.

### Example Clarification Prompts for Cursor

When working in Cursor, prepend context with:

> "Use the HashTimer hierarchy defined in `/docs/CURSOR_INSTRUCTIONS_HASHTIMERS.md`.
> Make sure transaction, block, and round timers are distinct structs, chained deterministically (Tx → Block → Round).
> Do not mix or flatten these types."

---

## 🧠 Example Command in Cursor

When you open a file (e.g. `crates/consensus/src/finality.rs`), you can issue:

> "Analyze this file and align all HashTimer references with `/docs/CURSOR_INSTRUCTIONS_HASHTIMERS.md`.
> Replace ambiguous `hash_timer` fields with explicit `TxHashTimer`, `BlockHashTimer`, or `RoundHashTimer` depending on context."

---

### ✅ Optional: reinforce with a comment in each relevant crate

Add at the top of each related module:

```rust
//! NOTE: This module uses the hierarchical HashTimer model (Tx → Block → Round).
//! See `/docs/CURSOR_INSTRUCTIONS_HASHTIMERS.md` for definitions.
```

