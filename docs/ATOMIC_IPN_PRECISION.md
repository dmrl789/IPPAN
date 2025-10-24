# Ultra-Fractional IPN Unit Architecture

## Executive Summary

IPPAN implements **24-decimal precision** for IPN tokens, enabling sub-yocto granularity for micro-payments, AI inference, IoT settlements, and machine-to-machine economies. This ultra-fine divisibility is critical for HashTimer-anchored micropayments at ~200ms round intervals, ensuring fairness across thousands of parallel blocks without rounding loss.

---

## 1. Technical Denomination

| Name       | Symbol    | Value in IPN | Atomic Units      | Typical Use Case                    |
|------------|-----------|--------------|-------------------|-------------------------------------|
| **IPN**    | IPN       | 1.0          | 10²⁴              | Governance, staking                 |
| **milli-IPN** | mIPN   | 0.001        | 10²¹              | Validator micro-rewards             |
| **micro-IPN** | µIPN   | 0.000001     | 10¹⁸              | Transaction fees                    |
| **nano-IPN**  | nIPN   | 0.000000001  | 10¹⁵              | Micro-services                      |
| **pico-IPN**  | pIPN   | 0.000000000001 | 10¹²            | Sub-cent settlements                |
| **femto-IPN** | fIPN   | 10⁻¹⁵        | 10⁹               | Streaming payments                  |
| **atto-IPN**  | aIPN   | 10⁻¹⁸        | 10⁶               | IoT energy metering                 |
| **zepto-IPN** | zIPN   | 10⁻²¹        | 10³               | AI micro-inference                  |
| **yocto-IPN** | yIPN   | 10⁻²⁴        | 1                 | **Atomic unit (smallest possible)** |

### Storage Format

- **Type**: `u128` (128-bit unsigned integer)
- **Smallest unit**: 1 yocto-IPN = 1 atomic unit = 10⁻²⁴ IPN
- **Total supply**: 21,000,000 IPN = 21,000,000 × 10²⁴ atomic units = 2.1 × 10³¹ units

```rust
/// Atomic unit type: 1 IPN = 10²⁴ atomic units
pub type AtomicIPN = u128;

/// Number of decimal places in IPN token
pub const IPN_DECIMALS: u32 = 24;

/// One full IPN token in atomic units
pub const ATOMIC_PER_IPN: AtomicIPN = 10u128.pow(24);
// = 1,000,000,000,000,000,000,000,000
```

---

## 2. Design Rationale

### 2.1 HashTimer Micro-Events

IPPAN's HashTimer produces deterministic rounds every ~200 milliseconds. Within each round:

- **Hundreds to thousands** of parallel blocks may be created
- Each block earns a fractional reward from the round pool
- Fine granularity prevents rounding errors and unnecessary burns

**Example**: If round reward R(t) = 0.0001 IPN and there are 1,000 blocks:
- Per-block reward = 0.0000001 IPN = 100,000,000,000,000,000 atomic units
- **No loss** from integer division

### 2.2 Parallel Block Reward Fairness

DAG-Fair Emission distributes rewards among all valid blocks in a round. Without sufficient precision:

- Small remainders would be lost or burned
- Validators contributing micro-blocks would receive nothing
- Economic incentives would favor larger validators

With 24-decimal precision:

- Every contribution, no matter how small, receives accurate compensation
- Remainder from division is minimal (typically < 1000 yocto-IPN)
- Can be explicitly burned or carried forward with full audit trail

### 2.3 Machine-to-Machine Economy

Devices and AI agents need to settle for:

- **Individual compute cycles** (nanosecond granularity)
- **Data packets** (bytes or KB)
- **Energy consumption** (millijoules or watts-per-second)
- **API calls** (per-token in LLM inference)

Traditional 8-decimal cryptocurrencies (like Bitcoin's satoshi) lack granularity for these use cases. With yocto-IPN:

- 1 API call = 10⁻¹² IPN = 1 trillion yocto-IPN
- 1 kWh energy = 10⁻⁶ IPN = 1 quintillion yocto-IPN
- Enables real-time micro-settlement without batching delays

### 2.4 Future-Proofing Scarcity

Even with a fixed supply of 21 million IPN:

- Total atomic units = 2.1 × 10³¹
- Global transactions per second capacity = billions
- No unit exhaustion for centuries at massive scale

---

## 3. Implementation Architecture

### 3.1 Core Types

```rust
/// Represents an amount of IPN in atomic units with overflow protection
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Amount(pub AtomicIPN);

impl Amount {
    /// Create from whole IPN tokens
    pub const fn from_ipn(ipn: u64) -> Self {
        Self((ipn as u128) * ATOMIC_PER_IPN)
    }

    /// Create from micro-IPN (common fee unit)
    pub const fn from_micro_ipn(micro: u64) -> Self {
        Self((micro as u128) * 1_000_000_000_000_000_000)
    }

    /// Create from raw atomic units
    pub const fn from_atomic(atomic: u128) -> Self {
        Self(atomic)
    }

    /// Convert to floating-point IPN (display only)
    pub fn to_ipn_f64(&self) -> f64 {
        self.0 as f64 / ATOMIC_PER_IPN as f64
    }

    /// Checked arithmetic prevents overflow
    pub fn checked_add(&self, other: Amount) -> Option<Amount>;
    pub fn checked_sub(&self, other: Amount) -> Option<Amount>;
    pub fn checked_mul(&self, scalar: u128) -> Option<Amount>;
    pub fn checked_div(&self, scalar: u128) -> Option<Amount>;

    /// Saturating arithmetic clamps to bounds
    pub fn saturating_add(&self, other: Amount) -> Amount;
    pub fn saturating_sub(&self, other: Amount) -> Amount;

    /// Calculate percentage (basis points: 10000 = 100%)
    pub fn percentage(&self, basis_points: u16) -> Amount {
        Amount((self.0 * basis_points as u128) / 10_000)
    }

    /// Split amount evenly among N recipients (returns per-recipient and remainder)
    pub fn split(&self, count: usize) -> (Amount, Amount) {
        if count == 0 {
            return (Amount::zero(), *self);
        }
        let per_recipient = self.0 / count as u128;
        let remainder = self.0 % count as u128;
        (Amount(per_recipient), Amount(remainder))
    }
}
```

### 3.2 Transaction Structure

All transaction amounts use the `Amount` type:

```rust
pub struct Transaction {
    pub id: [u8; 32],
    pub from: [u8; 32],
    pub to: [u8; 32],
    pub amount: Amount,  // ← 24-decimal atomic precision
    pub nonce: u64,
    // ... other fields
}
```

### 3.3 Emission & Reward Distribution

```rust
/// Initial round reward in micro-IPN
pub struct EmissionParams {
    pub r0: Amount,  // e.g., Amount::from_micro_ipn(10_000)
    pub halving_rounds: u64,
    pub supply_cap: Amount,  // 21M IPN
    // ...
}

/// Distribute round reward among proposer and verifiers
pub fn distribute_round_reward(
    round: u64,
    params: &EmissionParams,
    block_count: usize,
    verifier_count: usize,
) -> RoundRewardDistribution {
    let total = round_reward(round, params);
    let proposer_reward = total.percentage(params.proposer_bps);
    let verifier_pool = total.saturating_sub(proposer_reward);
    let per_verifier = verifier_pool / verifier_count as u128;
    // ...
}
```

**Key Properties**:
- All division returns exact atomic units
- Remainder is explicitly tracked and can be burned or recycled
- No floating-point drift or precision loss

### 3.4 Fee Caps

Fee limits are defined in atomic units:

```rust
pub struct FeeCapConfig {
    pub cap_transfer: Amount,       // Amount::from_micro_ipn(1_000)
    pub cap_ai_call: Amount,        // Amount::from_micro_ipn(100)
    pub cap_contract_deploy: Amount,// Amount::from_micro_ipn(100_000)
    // ...
}
```

This enables:
- **Pay-per-API-call** fees for AI inference (e.g., 0.0001 µIPN per token)
- **Micro-contract** executions at atto-IPN scale
- **IoT sensor** data submission at zepto-IPN granularity

---

## 4. Economic Consistency

| Property               | Effect                                                          |
|------------------------|-----------------------------------------------------------------|
| **No inflation**       | Total atomic supply = 21M × 10²⁴ units (fixed at genesis)       |
| **Rounding-safe**      | DAG-Fair emission distributes atomically; remainder auto-burned |
| **Deterministic**      | Fractional rewards computed via integer math — no float drift   |
| **Audit-ready**        | HashTimer proofs embed both round reward and sub-unit checksum  |
| **Backward compatible**| Old µIPN values map to new atomic units via: `old × 10¹⁶`       |

---

## 5. Practical Examples

### Example 1: Validator Reward Split

```
Round reward R(t) = 0.0001 IPN = 100,000,000,000,000,000,000 atomic units
Blocks in round B_r = 1,000
Per block = 100,000,000,000,000,000 atomic units = 0.0000001 IPN
```

Even the smallest block contributor receives a precise, non-zero amount.

### Example 2: AI Inference Payment

```
AI model charges 0.01 µIPN per token
User request = 500 tokens
Total fee = 5 µIPN = 5,000,000,000,000,000,000 atomic units
```

The payment is exact, with no rounding or minimum batch size required.

### Example 3: IoT Energy Settlement

```
Device consumes 0.5 watt-seconds
Rate = 0.000001 IPN per watt-second
Cost = 0.0000005 IPN = 500,000,000,000,000,000 atomic units
```

Real-time settlement without accumulating debt or batching.

---

## 6. Migration from Legacy Precision

If migrating from an 8-decimal µIPN system:

```rust
// Old: amount in µIPN (10⁸ per IPN)
let old_amount_micro_ipn: u64 = 1_000_000;  // 0.01 IPN

// New: convert to 24-decimal atomic units
let new_amount = Amount::from_atomic((old_amount_micro_ipn as u128) * 10u128.pow(16));
// = 1_000_000 × 10¹⁶ = 10,000,000,000,000,000,000,000 atomic units
// = 0.01 IPN in new system
```

**Conversion factor**: Multiply old µIPN values by **10¹⁶** to get atomic units.

---

## 7. Security & Validation

### 7.1 Overflow Protection

- All arithmetic uses **checked** or **saturating** operations
- Transactions attempting overflow are rejected at validation
- `u128` provides ~10³⁸ headroom above 21M IPN supply cap

### 7.2 Zero-Amount Detection

```rust
if amount.is_zero() {
    return Err(ValidationError::ZeroAmount);
}
```

Prevents dust attacks and ensures meaningful economic activity.

### 7.3 Signature Coverage

Transaction signatures include the full 128-bit amount:

```rust
fn message_bytes(&self) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&self.from);
    bytes.extend_from_slice(&self.to);
    bytes.extend_from_slice(&self.amount.atomic().to_be_bytes());  // ← 16 bytes
    // ...
}
```

This prevents amount tampering and ensures cryptographic integrity.

---

## 8. Display & User Interfaces

### 8.1 Human-Readable Format

By default, show up to **12 significant decimals**:

```rust
impl Display for Amount {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let ipn = self.0 / ATOMIC_PER_IPN;
        let fractional = self.0 % ATOMIC_PER_IPN;
        
        if fractional == 0 {
            write!(f, "{} IPN", ipn)
        } else {
            let trimmed = format!("{:024}", fractional).trim_end_matches('0');
            let truncated = if trimmed.len() > 12 { 
                &trimmed[..12] 
            } else { 
                trimmed 
            };
            write!(f, "{}.{} IPN", ipn, truncated)
        }
    }
}
```

**Examples**:
- `Amount::from_ipn(5)` → "5 IPN"
- `Amount::from_micro_ipn(123)` → "0.000123 IPN"
- `Amount::from_atomic(1)` → "0.000000000000000000000001 IPN"

### 8.2 Parsing from User Input

```rust
let amount = Amount::from_str_ipn("0.000001")?;  // 1 µIPN
let amount = Amount::from_str_ipn("1.5")?;        // 1.5 IPN
let amount = Amount::from_str_ipn("0.000000000000000000000001")?;  // 1 yocto-IPN
```

Accepts up to 24 decimal places; rejects larger precision.

---

## 9. Future Use Cases

### 9.1 AI Model Inference Pay-Per-Token

- **Scenario**: LLM charges 0.0001 µIPN per output token
- **Implementation**: `Amount::from_atomic(100_000_000_000_000)` per token
- **Benefits**: No batching, instant settlement, sub-cent pricing

### 9.2 Streaming Payments for Compute/Bandwidth

- **Scenario**: Cloud VM charges per millisecond of runtime
- **Implementation**: `Amount::from_atomic(rate_per_ms * elapsed_ms)`
- **Benefits**: Real-time billing, no overpayment, automatic refunds

### 9.3 IoT Device Micropayments

- **Scenario**: Smart meter reports energy consumption every 10 seconds
- **Implementation**: `Amount::from_atomic(energy_joules * rate_per_joule)`
- **Benefits**: Eliminates billing cycles, enables peer-to-peer energy trading

### 9.4 Cross-Chain Bridges with Sub-Cent Settlement

- **Scenario**: Bridge USDC with 0.0001 IPN fee per transfer
- **Implementation**: Fee = `Amount::from_micro_ipn(1)` (sub-penny precision)
- **Benefits**: Competitive with traditional payment networks, no minimum amounts

### 9.5 Autonomous Agent Micro-Tasks

- **Scenario**: AI agent pays another agent for data verification
- **Implementation**: `Amount::from_atomic(task_complexity * base_rate)`
- **Benefits**: Enables emergent agent economies without human oversight

---

## 10. Compliance & Auditability

### 10.1 Deterministic Emission

All emission calculations are **integer-only**:

```rust
// Halving schedule
let reward = params.r0.atomic() >> halvings;  // Bit shift = exact halving

// Percentage split
let proposer = total.percentage(2000);  // Exactly 20%
let verifier_pool = total.saturating_sub(proposer);
```

No floating-point operations → **reproducible across all platforms**.

### 10.2 Remainder Tracking

When splitting rewards, remainder is explicit:

```rust
let (per_verifier, remainder) = verifier_pool.split(verifier_count);

// Remainder options:
// 1. Burn (decrease supply)
// 2. Add to next round pool
// 3. Donate to treasury
```

All paths are auditable on-chain.

### 10.3 Supply Cap Enforcement

```rust
pub const SUPPLY_CAP: AtomicIPN = 21_000_000 * ATOMIC_PER_IPN;

assert!(projected_supply(rounds, &params) <= Amount(SUPPLY_CAP));
```

Hard cap prevents inflation; enforced at consensus level.

---

## 11. Performance Considerations

### 11.1 Storage Efficiency

- **Per-transaction overhead**: 16 bytes (u128) vs. 8 bytes (u64)
- **Trade-off**: +8 bytes per tx for 24-decimal precision
- **Acceptable**: Modern databases handle petabytes; 8 bytes negligible

### 11.2 Computational Cost

- u128 arithmetic is **native** on 64-bit CPUs (uses two registers)
- No performance penalty vs. u64 on modern hardware
- SIMD operations can process multiple u128 values in parallel

### 11.3 Network Serialization

- **Binary format**: 16 bytes (same as UUID)
- **JSON format**: String representation (e.g., `"1000000000000000000000000"`)
- **Compression**: High entropy limits compression gains; accept as-is

---

## 12. Integration Guide

### For Wallet Developers

```rust
// Display balance to user
println!("Your balance: {}", amount);  // Auto-formats to 12 decimals

// Parse user input
let send_amount = Amount::from_str_ipn(&user_input)?;

// Validate before sending
if send_amount > balance {
    return Err("Insufficient funds");
}
```

### For Exchange Integrators

```rust
// Deposit detection
let deposit = Amount::from_atomic(tx.amount.atomic());
credit_user_account(user_id, deposit);

// Withdrawal (convert from exchange internal units)
let withdrawal_amount = Amount::from_ipn(user_requested_ipn);
create_transaction(exchange_wallet, user_address, withdrawal_amount);
```

### For Smart Contract Developers

```rust
// Charge fee for contract execution
let fee = Amount::from_micro_ipn(10);  // 0.00001 IPN
require!(msg.value >= fee, "Insufficient payment");

// Split revenue among stakeholders
let (per_stakeholder, remainder) = total_revenue.split(stakeholder_count);
for stakeholder in stakeholders {
    transfer(stakeholder, per_stakeholder);
}
burn(remainder);  // Optional: burn dust
```

---

## 13. Testing & Validation

All precision features are covered by extensive unit tests:

```rust
#[test]
fn test_yocto_precision() {
    let one_yocto = Amount::from_atomic(1);
    assert_eq!(one_yocto.to_ipn_f64(), 1e-24);
}

#[test]
fn test_split_no_loss() {
    let total = Amount::from_atomic(999_999_999_999_999_999_999_999);
    let (per_unit, remainder) = total.split(1_000_000);
    let reconstructed = per_unit * 1_000_000 + remainder;
    assert_eq!(reconstructed, total);  // No rounding loss
}

#[test]
fn test_overflow_protection() {
    let max = Amount(u128::MAX);
    assert!(max.checked_add(Amount(1)).is_none());  // Overflows
    assert_eq!(max.saturating_add(Amount(1)), max);  // Clamps to max
}
```

---

## 14. Comparison with Other Systems

| System         | Decimals | Smallest Unit     | Total Units (21M cap) | Use Case Fit           |
|----------------|----------|-------------------|-----------------------|------------------------|
| **Bitcoin**    | 8        | 1 satoshi         | 2.1 × 10¹⁵            | Store of value         |
| **Ethereum**   | 18       | 1 wei             | 2.1 × 10²⁵            | Smart contracts        |
| **IPPAN**      | **24**   | **1 yocto-IPN**   | **2.1 × 10³¹**        | **AI/IoT micro-economy** |

IPPAN's 24-decimal precision is **1 million times finer** than Ethereum, enabling use cases impossible on traditional blockchains.

---

## 15. Conclusion

IPPAN's **ultra-fractional IPN unit architecture** is purpose-built for:

1. **High-frequency micro-rewards** in DAG-Fair emission
2. **HashTimer-anchored precision** at ~200ms intervals
3. **Machine-to-machine economies** (AI, IoT, DePIN)
4. **Future-proof scalability** with 2.1 × 10³¹ atomic units

By using 24-decimal precision with 128-bit integers, IPPAN achieves:

- ✅ **Zero rounding loss** in reward distribution
- ✅ **Sub-cent payment granularity** for micro-services
- ✅ **Deterministic, audit-friendly** arithmetic
- ✅ **Backward compatibility** via simple conversion factor

This design positions IPPAN as the **premier L1 for the machine economy**, where billions of devices and agents transact autonomously at yocto-IPN precision.

---

## Appendix: Reference Implementation

See `/workspace/crates/types/src/currency.rs` for complete implementation, including:

- `Amount` type with checked/saturating operations
- Denomination constants (IPN, mIPN, µIPN, ... yIPN)
- Parsing from decimal strings
- Display formatting with configurable precision
- Comprehensive test suite

For emission logic, see `/workspace/crates/consensus/src/emission.rs`.
For fee validation, see `/workspace/crates/consensus/src/fees.rs`.

---

**Document Version**: 1.0  
**Last Updated**: 2025-10-23  
**Maintainer**: IPPAN Core Team  
**License**: MIT
