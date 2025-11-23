# Fee Caps and DAG-Fair Emission System

This document defines IPPAN’s **fee system**, **DAG-Fair emission schedule**, and **reward distribution model**.  
It merges the formal protocol specification (for implementation) with extended economic explanations (for the whitepaper and governance reference).

---

## 1. Fee System

### 1.1 Overview

IPPAN enforces **hard, deterministic fee caps** at the protocol level to ensure predictable costs, fairness, and resistance to manipulation.  
Transactions that exceed their allowed fee cap are **rejected deterministically** during both **mempool admission** and **block assembly**, making fee behavior transparent and uniform across all nodes.

This guarantees:
- 💸 Predictable and capped transaction costs  
- 🛡️ Protection against fee-based centralization  
- ⚖️ Fair treatment of all transaction classes  

### 1.2 Fee Cap Table

| Transaction Type     | Cap (µIPN) | Cap (IPN)   | Description                     |
|----------------------|------------|-------------|----------------------------------|
| Transfer             | 1,000      | 0.000001    | Simple token transfer            |
| AI Call              | 100        | 0.0000001   | AI model inference call          |
| Contract Deploy      | 100,000    | 0.0001      | Deploy smart contract            |
| Contract Call        | 10,000     | 0.00001     | Execute contract method          |
| Governance           | 10,000     | 0.00001     | Governance proposal or vote      |
| Validator Operations | 10,000     | 0.00001     | Stake, register, or update node  |

> *1 IPN = 1,000,000 µIPN (with 24-decimal precision)*

### 1.3 Fee Validation

Each transaction’s fee is checked using deterministic validation logic during admission:

```rust
for tx in block.transactions {
    validate_fee(&tx, tx.fee, &fee_config)?;
}
```

---

## 2. Ultra-Fractional IPN Units

### 2.1 Technical Denomination

IPPAN supports ultra-fine divisibility to enable **HashTimer-anchored micropayments** and **DAG-Fair emission** with atomic precision. The system uses a 24-decimal fixed-point representation for maximum granularity.

| Name     | Symbol    | Value in IPN | Typical Use                                                 |
|----------|-----------|--------------|-------------------------------------------------------------|
| **IPN**  | 1 IPN     | 1.0          | governance, staking                                         |
| **mIPN** | milli-IPN | 0.001        | validator micro-rewards                                     |
| **µIPN** | micro-IPN | 0.000001     | transaction fees                                            |
| **aIPN** | atto-IPN  | 10⁻¹⁸        | IoT, AI micro-service calls                                 |
| **zIPN** | zepto-IPN | 10⁻²¹        | sub-millisecond AI or machine-to-machine triggers           |
| **yIPN** | yocto-IPN | 10⁻²⁴        | theoretical lower limit, HashTimer precision-level payments |

> ✅ **Smallest accepted fraction:**
> `1 yIPN = 0.000000000000000000000001 IPN`
> (1 × 10⁻²⁴ IPN)

That's **one septillionth** of an IPN — still representable with a 128-bit fixed-point integer (e.g. `u128` with 24 decimal precision).

### 2.2 Why Such Extreme Precision?

1. **HashTimer micro-events**: Rounds occur every ~200 ms, so actions (AI inference, sensor reports, cross-device syncs) can trigger sub-microsecond payments.

2. **Parallel block reward fairness**: A single validator round may distribute rewards among **thousands** of micro-blocks. Fine granularity avoids rounding errors and unnecessary burns.

3. **Machine-to-machine economy**: Devices can settle for infinitesimal compute, data, or energy units — ideal for **DePIN**, **IoT**, and **AI agent** economies.

4. **Future-proofing scarcity**: Even with 21 M IPN total supply, the system supports trillions of atomic transactions per second without unit exhaustion.

### 2.3 Implementation Detail

In code (Rust):

```rust
/// IPN is stored as fixed-point integer with 24 decimal places.
/// 1 IPN = 10^24 atomic units.
pub type AtomicIPN = u128;

pub const IPN_DECIMALS: u32 = 24;
pub const ATOMIC_PER_IPN: AtomicIPN = 10u128.pow(IPN_DECIMALS);

/// Example: convert 0.000000000000000000000001 IPN to atomic units
let one_yocto = 1u128; // 1 atomic unit
```

All ledger, wallet, and transaction components handle balances and fees at this **atomic precision**, while human-readable interfaces show up to 8–12 decimals by default.

### 2.4 Economic Consistency

| Property          | Effect                                                          |
|-------------------|------------------------------------------------------------------|
| **No inflation**  | Total atomic supply = 21 M × 10²⁴ units (fixed)                 |
| **Rounding-safe** | DAG-Fair emission distributes atomically, remainder auto-burned |
| **Deterministic** | Fractional rewards computed via integer math — no float drift   |
| **Audit-ready**   | HashTimer proofs embed both round reward and sub-unit checksum  |

### 2.5 Example — Validator Reward Split

```
Round reward R(t) = 0.0001 IPN = 10^20 atomic units
Blocks in round B_r = 1,000
Per block = 10^17 atomic units = 0.0000000000001 IPN
```

Even the smallest block contributor still receives a precise amount — no loss of accuracy, no unfair truncation.

### 2.6 Future Use Cases

* **AI model inference pay-per-token**
* **Streaming payments** for compute or bandwidth
* **IoT energy metering** per joule or data packet
* **Cross-chain bridges** with sub-cent settlement
* **Autonomous agents** performing micro-tasks and paying instantly

### 2.7 Summary

| Feature           | Description                                              |
|-------------------|----------------------------------------------------------|
| **Divisibility**  | up to 10⁻²⁴ IPN (yocto-IPN precision)                    |
| **Storage type**  | 128-bit fixed-point integer                              |
| **Fairness**      | no rounding loss across billions of micro-rewards        |
| **Compatibility** | fits HashTimer precision, 10–50 ms block interval design |
| **Use cases**     | AI, IoT, micro-services, DePIN                           |

---

## 3. DAG-Fair Emission System

### Weekly redistribution and no-burn handling

- 5% of each round’s emission is reserved for the network dividend pool.
- 25% of collected fees are paid immediately in the round; the remaining 75%
  (including any amount above the per-round cap) is routed to the pool.
- The pool is redistributed on the weekly audit cadence, ensuring every fee and
  dividend slice returns to participants instead of being burned.
