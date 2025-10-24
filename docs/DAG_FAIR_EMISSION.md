# DAG-Fair Emission System

> **Deterministic, Round-Based Monetary Policy for IPPAN BlockDAG**

---

## Overview

IPPAN implements a **DAG-Fair Emission** system designed specifically for high-throughput BlockDAG architectures. Unlike traditional blockchains that emit rewards per block, IPPAN anchors emission to **rounds** — deterministic time windows (≈100–250 ms) that aggregate thousands of parallel micro-blocks.

This design ensures:
- **Predictable monetary policy** with a 21M IPN cap
- **Fair distribution** across all validators regardless of hardware
- **Scalability** — emission rate independent of block production speed
- **Auditability** — full transparency via HashTimer™ synchronization

---

## Core Design Principles

### 1. Round-Based Emission

**Problem:** In IPPAN's BlockDAG, thousands of blocks are produced per second. Per-block rewards would cause hyperinflation.

**Solution:** Emission is tied to **rounds**, not blocks. Each round (≈100ms) has a **fixed total reward** distributed among all validators who contributed during that round.

```
Round Reward = R₀ / 2^⌊round / halving_interval⌋
```

Where:
- `R₀ = 0.0001 IPN` (10,000 µIPN) — initial reward per round
- `halving_interval = 630,720,000 rounds` (≈2 years at 100ms/round)

### 2. Proportional Fairness

Validators receive rewards proportional to their **weighted contribution**:

```
Contribution Score = (blocks_proposed × proposer_weight + blocks_verified × verifier_weight)
                    × (reputation / 10000)
                    × (uptime / 10000)
```

This ensures:
- High-performance validators earn more
- Proposers get a bonus (default 1.2×) for block creation
- Reputation and uptime impact earnings
- No validator can dominate through hardware alone

### 3. Multi-Component Rewards

Each round's total reward comes from four sources:

| Component | Default Share | Source |
|-----------|--------------|---------|
| **Base Emission** | 60% | Protocol-defined emission schedule |
| **Transaction Fees** | 25% | Fees collected from transactions in the round |
| **AI Service Commissions** | 10% | Revenue from AI inference marketplace |
| **Network Dividend** | 5% | Weekly redistribution from reward pool |

---

## Emission Schedule

### Parameters

| Parameter | Value | Description |
|-----------|-------|-------------|
| **Total Supply Cap** | 21,000,000 IPN | Hard cap (never exceeded) |
| **Initial Reward** | 0.0001 IPN/round | 10,000 µIPN per round |
| **Round Duration** | ~100 ms | 10 rounds/second average |
| **Annual Rounds** | ~315,360,000 | Deterministic |
| **Halving Interval** | 630,720,000 rounds | ~2 years |

### Emission Curve

| Year | Reward/Round | Annual Issuance | Cumulative Supply |
|------|--------------|-----------------|-------------------|
| 1-2  | 0.0001 IPN   | ~31,536 IPN     | 31,536 IPN        |
| 3-4  | 0.00005 IPN  | ~15,768 IPN     | 47,304 IPN        |
| 5-6  | 0.000025 IPN | ~7,884 IPN      | 55,188 IPN        |
| 7-8  | 0.0000125 IPN| ~3,942 IPN      | 59,130 IPN        |
| 9-10 | 0.00000625 IPN| ~1,971 IPN     | 61,101 IPN        |

After 10 halvings (≈20 years), emission becomes negligible, with the system sustained by transaction fees and AI service commissions.

### Projected Supply Growth

```rust
// Example: Calculate supply after 1 year
let params = EmissionParams::default();
let year1_rounds = 315_360_000; // 1 year
let supply = projected_supply(year1_rounds, &params);
// Result: ~31,536 IPN (0.15% of total cap)
```

---

## Distribution Algorithm

### Step 1: Calculate Round Reward

```rust
fn round_reward(round: u64, params: &EmissionParams) -> u128 {
    let halvings = round / params.halving_rounds;
    params.r0 >> halvings  // Bit-shift for efficient division by 2^halvings
}
```

### Step 2: Collect Contributions

For each validator in the round:

```rust
struct ValidatorContribution {
    validator_id: [u8; 32],
    blocks_proposed: usize,
    blocks_verified: usize,
    reputation_score: u32,  // 0-10000 (10000 = 100%)
    uptime_factor: u32,     // 0-10000
}
```

### Step 3: Calculate Weighted Scores

```rust
weighted_score = (blocks_proposed × proposer_weight + blocks_verified × verifier_weight)
                × (reputation / 10000)
                × (uptime / 10000)
```

### Step 4: Distribute Proportionally

```rust
validator_reward = total_round_reward × (validator_score / sum_all_scores)
```

---

## Fee Integration

### Fee Caps

IPPAN enforces **protocol-level fee caps** to prevent congestion pricing:

| Transaction Type | Max Fee (µIPN) | USD Equivalent* |
|------------------|---------------|-----------------|
| Transfer         | 1,000         | $0.0001         |
| AI Call          | 100           | $0.00001        |
| Contract Deploy  | 100,000       | $0.01           |
| Contract Call    | 10,000        | $0.001          |
| Governance       | 10,000        | $0.001          |

*Assuming 1 IPN = $1 for illustration

### Fee Recycling

Collected fees are recycled back into the reward system:

1. **25%** of fees → immediate round rewards
2. **75%** → network reward pool
3. Pool dividends distributed weekly to active validators

---

## Emission Tracking & Auditing

### EmissionTracker

The `EmissionTracker` maintains full emission state:

```rust
pub struct EmissionTracker {
    pub cumulative_supply: u128,
    pub last_round: u64,
    pub total_fees_collected: u128,
    pub total_ai_commissions: u128,
    pub network_pool_balance: u128,
    pub validator_earnings: HashMap<[u8; 32], u128>,
    pub audit_history: Vec<EmissionAuditRecord>,
}
```

### Audit Checkpoints

Every N rounds (default: weekly), the system creates an audit record:

```rust
pub struct EmissionAuditRecord {
    pub start_round: u64,
    pub end_round: u64,
    pub total_base_emission: u128,
    pub total_fees_collected: u128,
    pub cumulative_supply: u128,
    pub distribution_hash: [u8; 32],
}
```

**Purpose:**
- Governance verification
- External audits
- Consensus validation
- Fraud detection

### Consistency Verification

The tracker verifies consistency at each checkpoint:

```rust
fn verify_consistency(&self) -> Result<(), String> {
    let expected = projected_supply(self.last_round, &self.params);
    let tolerance = self.last_round as u128; // Allow small rounding
    
    if self.cumulative_supply > expected + tolerance {
        return Err("Excess emission detected");
    }
    
    Ok(())
}
```

---

## Governance Controls

### On-Chain Parameters

The following parameters can be adjusted via governance vote:

- `proposer_weight_bps` — Proposer reward multiplier
- `verifier_weight_bps` — Verifier reward multiplier
- Component share percentages (base/fee/AI/dividend split)
- Audit checkpoint interval

**Immutable parameters** (require hard fork):
- Total supply cap (21M IPN)
- Halving interval
- Initial reward (R₀)

### Upgrade Process

1. Proposal submitted to governance contract
2. Validator voting period (≥ 7 days)
3. Super-majority required (>66%)
4. Activation delay (≥ 30 days)
5. Parameters updated via consensus

---

## Security & Anti-Gaming

### Sybil Resistance

- Validators must stake minimum IPN
- Reputation system tracks long-term behavior
- New validators start with neutral reputation (50%)
- Slashing for malicious behavior

### Fairness Guarantees

1. **Temporal Fairness:** All blocks in a round share the same reward pool
2. **Hardware Neutrality:** Fast nodes can't out-compete slower nodes within a round
3. **Proportional Rewards:** Earnings scale linearly with contribution
4. **No Front-Running:** Round boundaries enforced by HashTimer™

### Supply Cap Protection

```rust
// Hard limit enforcement
if cumulative_supply + round_reward > supply_cap {
    return Err("Supply cap reached");
}

// Automatic burn of excess (safety margin)
let excess = cumulative_supply.saturating_sub(supply_cap);
if excess > 0 {
    burn_excess(excess);
}
```

---

## Implementation Details

### Key Modules

```
crates/consensus/src/
├── emission.rs              # Core emission formulas
├── emission_tracker.rs      # State tracking & auditing
├── fees.rs                  # Fee validation & recycling
└── reputation.rs            # AI-based validator scoring
```

### Public API

```rust
// Calculate reward for a specific round
pub fn round_reward(round: u64, params: &EmissionParams) -> u128;

// Project total supply after N rounds
pub fn projected_supply(rounds: u64, params: &EmissionParams) -> u128;

// Distribute rewards among validators
pub fn distribute_round_reward(
    round: u64,
    params: &EmissionParams,
    contributions: &[ValidatorContribution],
    transaction_fees: u128,
    ai_commissions: u128,
    network_pool_balance: u128,
) -> RoundRewardDistribution;

// Track emission over time
pub struct EmissionTracker {
    pub fn process_round(&mut self, ...) -> Result<RoundRewardDistribution>;
    pub fn verify_consistency(&self) -> Result<()>;
    pub fn get_statistics(&self) -> EmissionStatistics;
}
```

---

## Testing & Validation

### Test Coverage

The emission system includes comprehensive tests:

- **Unit Tests:** Formula correctness, edge cases
- **Integration Tests:** Multi-round scenarios, fairness validation
- **Property Tests:** Supply cap never exceeded, monotonic growth
- **Stress Tests:** Long-term projections (100+ years)

Run tests:

```bash
cargo test -p ippan-consensus --lib emission
cargo test -p ippan-consensus --test emission_integration_tests
```

### Validation Checklist

Before deploying emission changes:

- [ ] Parameters validate via `params.validate()`
- [ ] Supply projection < cap for all rounds
- [ ] Consistency verification passes
- [ ] Distribution sums equal total reward
- [ ] Audit trail generation works
- [ ] Governance approval obtained

---

## Migration & Compatibility

### Upgrading From Legacy Systems

If migrating from a per-block reward system:

1. Calculate equivalent round reward:
   ```
   R₀ = (block_reward × blocks_per_round) / validator_count
   ```

2. Preserve cumulative supply:
   ```rust
   tracker.cumulative_supply = previous_system_total_emitted;
   ```

3. Migrate validator earnings:
   ```rust
   for (validator_id, earnings) in legacy_earnings {
       tracker.validator_earnings.insert(validator_id, earnings);
   }
   ```

### Backwards Compatibility

Emission records are compatible with:
- Legacy block explorers (via synthetic "emission blocks")
- Accounting systems (µIPN → IPN conversion: divide by 10⁸)
- Tax reporting (validator earnings queryable by round range)

---

## Examples

### Example 1: Single Round Distribution

```rust
use ippan_consensus::*;

let params = EmissionParams::default();

let contributions = vec![
    ValidatorContribution {
        validator_id: [1u8; 32],
        blocks_proposed: 10,
        blocks_verified: 20,
        reputation_score: 10000,  // 100%
        uptime_factor: 10000,     // 100%
    },
    ValidatorContribution {
        validator_id: [2u8; 32],
        blocks_proposed: 5,
        blocks_verified: 15,
        reputation_score: 8000,   // 80%
        uptime_factor: 9500,      // 95%
    },
];

let distribution = distribute_round_reward(
    100,        // round number
    &params,
    &contributions,
    1_000,      // transaction fees (µIPN)
    500,        // AI commissions (µIPN)
    10_000,     // network pool balance (µIPN)
);

println!("Total distributed: {} µIPN", distribution.total_distributed);
for (validator_id, reward) in &distribution.validator_rewards {
    println!("Validator {:?}: {} µIPN", validator_id, reward);
}
```

### Example 2: Long-Term Projection

```rust
use ippan_consensus::*;

let params = EmissionParams::default();

// Project supply after 10 years
let rounds_per_year = 315_360_000u64;
let years = 10;
let supply = projected_supply(rounds_per_year * years, &params);

println!("Supply after {} years: {} IPN", years, supply / 100_000_000);
println!("Percentage of cap: {:.2}%", 
         (supply as f64 / params.supply_cap as f64) * 100.0);
```

### Example 3: Emission Tracking

```rust
use ippan_consensus::*;

let params = EmissionParams::default();
let mut tracker = EmissionTracker::new(params, 6_048_000); // Weekly audits

// Process 1000 rounds
for round in 1..=1000 {
    let contributions = get_round_contributions(round);
    let fees = get_round_fees(round);
    let commissions = get_ai_commissions(round);
    
    tracker.process_round(round, &contributions, fees, commissions)?;
}

// Get statistics
let stats = tracker.get_statistics();
println!("Cumulative supply: {} IPN", stats.cumulative_supply / 100_000_000);
println!("Active validators: {}", stats.active_validators);
println!("Emission progress: {:.2}%", stats.percentage_emitted as f64 / 100.0);

// Verify consistency
tracker.verify_consistency()?;
```

---

## FAQ

### Q: Why rounds instead of blocks?

**A:** IPPAN produces thousands of blocks per second in parallel. Rewarding each block would cause hyperinflation. Rounds provide a stable time-based anchor for emission.

### Q: How are round boundaries determined?

**A:** Round boundaries are enforced by HashTimer™, IPPAN's deterministic time oracle. All nodes agree on round transitions within microseconds.

### Q: What happens if no blocks are produced in a round?

**A:** The round reward is not emitted. This preserves the supply cap and ensures rewards only go to active validators.

### Q: Can validators game the system by proposing many low-value blocks?

**A:** No. Rewards are distributed by round, not per block. Creating more blocks within a round doesn't increase that round's total reward.

### Q: How does reputation affect earnings?

**A:** Low reputation validators receive proportionally less. A validator with 50% reputation earns half as much as a 100% reputation validator with identical block production.

### Q: What happens after the supply cap is reached?

**A:** Base emission stops. Validators continue earning from transaction fees (25% share), AI commissions (10%), and network dividends (5%).

### Q: Is the emission schedule guaranteed?

**A:** The total cap and halving interval are immutable (would require hard fork). Component shares can be adjusted via governance with super-majority approval.

---

## References

1. **IPPAN Whitepaper:** Detailed BlockDAG architecture
2. **HashTimer™ Specification:** Deterministic time synchronization
3. **AI Reputation System:** Validator scoring methodology
4. **Fee Market Design:** Protocol-level cap enforcement
5. **Governance Framework:** Parameter upgrade process

---

## Conclusion

IPPAN's DAG-Fair Emission system solves the unique challenges of reward distribution in high-throughput BlockDAG networks:

✅ **Predictable monetary policy** — 21M cap with Bitcoin-like halvings  
✅ **Fair distribution** — Proportional to contribution, not hardware  
✅ **Scalable** — Emission independent of block production rate  
✅ **Auditable** — Full transparency via on-chain records  
✅ **Sustainable** — Fee and AI revenue post-emission  

The system is **production-ready**, **thoroughly tested**, and **governance-controlled**.

For implementation details, see:
- `crates/consensus/src/emission.rs`
- `crates/consensus/src/emission_tracker.rs`
- `crates/consensus/tests/emission_integration_tests.rs`

---

**Last Updated:** 2025-10-23  
**Status:** ✅ Implemented & Tested  
**Activation:** Pending Governance Approval
