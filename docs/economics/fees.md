# IPPAN Fee Policy

This document describes the complete fee policy for IPPAN, including fee computation, routing, and redistribution.

## Core Principles

1. **Fees are NEVER burned** - All fees go to participants
2. **Weekly redistribution pool** - Deterministic epoch-based distribution
3. **No floating point** - All math uses integer kambei (u64) or u128 intermediates
4. **Governance-updatable** - Fee schedules can be changed via proposals with timelock

## Fee Schedule (v1)

### Transaction Fees

Fees are computed using `FeeScheduleV1`:

```rust
pub struct FeeScheduleV1 {
    pub version: u32,
    pub tx_base_fee: u64,      // Base fee per transaction (kambei)
    pub tx_byte_fee: u64,      // Per-byte fee (kambei)
    pub tx_min_fee: u64,       // Minimum fee floor (kambei)
    pub tx_max_fee: u64,       // Maximum fee cap (kambei)
}
```

#### Default Values

| Field | Default (kambei) | In IPN |
|-------|------------------|--------|
| tx_base_fee | 100 | 0.000001 |
| tx_byte_fee | 1 | 0.00000001 |
| tx_min_fee | 100 | 0.000001 |
| tx_max_fee | 1,000,000,000 | 10 |

#### Fee Computation Formula

```
fee = max(tx_min_fee, tx_base_fee + tx_byte_fee * tx_bytes)
fee = clamp(fee, tx_min_fee, tx_max_fee)
```

### Handle/Domain Registry Fees

Registry operations use `RegistryPriceScheduleV1`:

```rust
pub struct RegistryPriceScheduleV1 {
    pub version: u32,
    pub handle_register_fee: u64,
    pub handle_update_fee: u64,
    pub handle_renew_fee_per_year: u64,
    pub handle_transfer_fee: u64,
    pub premium_handle_multiplier_bps: u16,  // 20000 = 2x
    pub domain_register_fee: u64,
    pub domain_renew_fee_per_year: u64,
}
```

#### Default Registry Prices

| Operation | Fee (kambei) | In IPN |
|-----------|--------------|--------|
| Handle Registration | 1,000,000 | 0.01 |
| Handle Update | 100,000 | 0.001 |
| Handle Renewal (per year) | 500,000 | 0.005 |
| Handle Transfer | 100,000 | 0.001 |
| Premium Handle Multiplier | 2x | - |
| Domain Registration | 10,000,000 | 0.1 |
| Domain Renewal (per year) | 5,000,000 | 0.05 |

### Premium Handles

Handles with premium TLDs (`.cyborg`, `.iot`, `.m`) incur a multiplier:

```rust
// Premium TLDs cost 2x base registration fee
pub fn is_premium(handle: &str) -> bool {
    matches!(handle.tld(), Some("cyborg") | Some("iot") | Some("m"))
}
```

## Weekly Fee Pool

### Pool Architecture

All fees flow into a deterministic weekly pool:

```
┌─────────────┐     ┌──────────────┐     ┌─────────────────┐
│ Transaction │────>│ Fee Pool     │────>│ Validator       │
│ Fees        │     │ (Epoch N)    │     │ Distribution    │
└─────────────┘     └──────────────┘     └─────────────────┘
                           │
┌─────────────┐            │
│ Registry    │────────────┘
│ Fees        │
└─────────────┘
```

### Pool Account ID

Each epoch has a deterministic **system account** with **no private key**:

```rust
/// Domain separation prefix for fee pool derivation
const FEE_POOL_PREFIX: &[u8] = b"FEE_POOL";

pub fn fee_pool_account_id(epoch: Epoch) -> [u8; 32] {
    BLAKE3(FEE_POOL_PREFIX || epoch.to_le_bytes())
}
```

**Key properties:**
- Pool is a **system account** — no private key exists
- Pool funds can **only** exit via the `DistributeFees` epoch-close transition
- Deterministic: same epoch always derives same account ID

### Epoch Definition

```rust
pub const EPOCH_SECS: u64 = 7 * 24 * 60 * 60;  // 7 days
pub const EPOCH_US: u64 = EPOCH_SECS * 1_000_000;

pub fn epoch_from_ippan_time_us(t_us: u64) -> Epoch {
    t_us / EPOCH_US
}
```

## Fee Routing

### During Transaction Execution

1. Compute fee: `schedule.compute_tx_fee(tx_bytes)`
2. Deduct from sender balance (checked >=)
3. Credit to `fee_pool_account_id(current_epoch)`
4. Track in `WeeklyFeePoolManager`

```rust
pub fn route_fee_to_pool(
    pool_manager: &WeeklyFeePoolManager,
    fee: Kambei,
    ledger: &mut dyn AccountLedger,
) -> Result<()>
```

### Failure Charging Policy

| Failure Category | Fee Charged |
|-----------------|-------------|
| Parse failure | None |
| Signature failure | None |
| Invalid nonce | tx_min_fee (if payable) |
| Insufficient balance | tx_min_fee (if payable) |
| State conflict | tx_base_fee (if payable) |
| Registry failure | tx_base_fee (if payable) |
| Execution failure | Computed fee |

## Weekly Redistribution

### Distribution Transition

At epoch boundary, the protocol executes `DistributeFees`:

```rust
pub struct DistributeFeesTransition {
    pub epoch: Epoch,
    pub execution_round: u64,
}
```

### Eligibility Criteria

Validators must meet **strict uninterrupted presence** to be eligible:

```rust
/// Uninterrupted presence required (100% = 10000 bps)
pub const MIN_PRESENCE_BPS: u16 = 10000;
pub const WORK_SCORE_CAP: u64 = 10_000;  // Anti-whale

/// Eligibility requires:
/// 1. Uninterrupted presence (100%)
/// 2. Not slashed during epoch
/// 3. work_score > 0
pub fn is_eligible(&self) -> bool {
    self.uninterrupted_presence && !self.slashed && self.work_score > 0
}

pub fn weight(&self) -> u64 {
    if !self.is_eligible() { return 0; }
    min(self.work_score, WORK_SCORE_CAP)
}
```

**Important**: A validator with 99% presence is **NOT eligible**. Only 100% uninterrupted presence qualifies.

### Payout Formula

```
payout_i = pool_balance * weight_i / sum(all_weights)
```

Using integer math with remainder tracking:

```rust
let payout = mul_div_u128(
    pool_balance,
    validator_weight as u128,
    total_weight
)?;
```

### Remainder Handling

Remainder from integer division carries to the next epoch pool:

```rust
let remainder = pool_balance - sum(payouts);
// remainder goes to fee_pool_account_id(epoch + 1)
```

### Payout Profiles

Validators can split payouts across up to 8 addresses:

```rust
pub struct PayoutProfile {
    pub recipients: Vec<PayoutRecipient>,  // max 8
}

pub struct PayoutRecipient {
    pub address: [u8; 32],
    pub weight_bps: u16,  // must sum to 10000
}
```

## Invariants

1. **Conservation**: `sum(payouts) + remainder = pool_balance`
2. **Single Distribution**: Each epoch can only be distributed once
3. **Protocol-only Spend**: Pool funds can only leave via `DistributeFees` transition
4. **Deterministic**: Same state + metrics = same distribution
5. **Uninterrupted Presence**: Only validators with 100% presence are eligible
6. **No Burning**: Fees are NEVER burned — all go to the weekly pool

## Governance

Fee schedules are governance-updatable with:
- **NOTICE_EPOCHS = 2** (typically 2 weeks)
- **QUORUM = 60%** of voting power must participate
- **APPROVAL = 2/3** (66.67%) of decisive votes must be "yes"
- **±20%/epoch bounds** on parameter changes
- **Minimum fee floors** (>= 1 kambei)

## Implementation

- **Types**: `crates/types/src/fee_policy.rs`
- **L1 Fees**: `crates/l1_fees/src/lib.rs`
- **Pool Manager**: `crates/treasury/src/weekly_pool.rs`
- **Handle Fees**: `crates/consensus/src/handles.rs`
- **Governance**: `crates/governance/src/fee_schedule.rs`

## Related Documentation

- [Units](units.md) - Kambei and AtomicIPN precision
- [Parameter Changes](../governance/parameter_changes.md) - Governance process
- [DAG-Fair Emission](../DAG_FAIR_EMISSION.md) - Emission schedule
