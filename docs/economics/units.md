# IPPAN Currency Units and Precision

This document defines the canonical units and precision used throughout the IPPAN protocol for fee computation, balances, and rewards.

## Overview

IPPAN uses two complementary unit systems:

1. **AtomicIPN (u128, 24 decimals)**: Ultra-high precision for micro-payments, AI inference, and IoT
2. **Kambei (u64, 8 decimals)**: Bitcoin-style precision for fees and governance

Both systems coexist to serve different use cases while maintaining deterministic integer math.

## Kambei Unit System (8 Decimals)

The **kambei** is the primary unit for fee computation and governance operations.

```
1 IPN = 100,000,000 kambei (10^8)
```

This follows Bitcoin's satoshi model and provides:
- Simple integer arithmetic
- Sufficient precision for transaction fees
- Compatibility with most financial operations

### Kambei Constants

```rust
pub const KAMBEI_PER_IPN: u64 = 100_000_000;
```

### Conversion Functions

```rust
// IPN to kambei
pub fn ipn_to_kambei(ipn: u64) -> u64 {
    ipn.saturating_mul(KAMBEI_PER_IPN)
}

// Kambei to IPN (truncates fractional part)
pub fn kambei_to_ipn(kambei: u64) -> u64 {
    kambei / KAMBEI_PER_IPN
}
```

## AtomicIPN Unit System (24 Decimals)

For ultra-fine precision required by:
- HashTimer micro-events (~200ms rounds)
- AI inference pay-per-token
- IoT energy metering
- Machine-to-machine micropayments

```
1 IPN = 10^24 atomic units (yocto-IPN)
```

### Denomination Table

| Name     | Symbol | Value in IPN  | Atomic Units |
|----------|--------|---------------|--------------|
| IPN      | IPN    | 1.0           | 10^24        |
| milli-IPN| mIPN   | 0.001         | 10^21        |
| micro-IPN| µIPN   | 0.000001      | 10^18        |
| nano-IPN | nIPN   | 10^-9         | 10^15        |
| pico-IPN | pIPN   | 10^-12        | 10^12        |
| femto-IPN| fIPN   | 10^-15        | 10^9         |
| atto-IPN | aIPN   | 10^-18        | 10^6         |
| zepto-IPN| zIPN   | 10^-21        | 10^3         |
| yocto-IPN| yIPN   | 10^-24        | 1            |

## Usage Guidelines

### When to Use Kambei

- Transaction fee computation
- Handle/domain registration fees
- Governance proposal fees
- Validator reward calculations
- Weekly fee pool distribution

### When to Use AtomicIPN

- Balance storage in ledger
- Emission curve calculations
- Precision-critical micropayments
- DAG-Fair reward distribution
- Internal accounting

## Mathematical Operations

### Safe Integer Math

All fee calculations use checked integer operations:

```rust
/// Safe multiplication and division using u128 intermediate
pub fn mul_div_u128(n: u128, mul: u128, div: u128) -> Option<u128> {
    if div == 0 { return None; }
    n.checked_mul(mul).map(|product| product / div)
}

/// Checked addition for kambei
pub fn checked_add_kambei(a: u64, b: u64) -> Option<u64> {
    a.checked_add(b)
}

/// Checked subtraction for kambei
pub fn checked_sub_kambei(a: u64, b: u64) -> Option<u64> {
    a.checked_sub(b)
}
```

### Invariants

1. **No floating point**: All consensus-level math uses integers only
2. **Overflow protection**: Checked operations prevent silent overflow
3. **Rounding transparency**: Remainder from division is always tracked
4. **Determinism**: Same inputs always produce same outputs across nodes

## Fee Precision

Transaction fees use kambei precision:

| Fee Type | Typical Range (kambei) | In IPN |
|----------|------------------------|--------|
| Minimum Fee | 100 | 0.000001 |
| Base Fee | 100 | 0.000001 |
| Per-Byte Fee | 1 | 0.00000001 |
| Handle Registration | 1,000,000 | 0.01 |
| Domain Registration | 10,000,000 | 0.1 |

## Implementation Location

- **Kambei types**: `crates/types/src/fee_policy.rs`
- **AtomicIPN types**: `crates/types/src/currency.rs`
- **Fee computation**: `crates/l1_fees/src/lib.rs`

## Related Documentation

- [Fee Policy](fees.md) - Detailed fee computation and routing
- [Parameter Changes](../governance/parameter_changes.md) - Governance updates to fee schedules
- [Tokenomics](../protocol/TOKENOMICS.md) - Overall economic model
