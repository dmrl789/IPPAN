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
| Transfer             | 1,000      | 0.00001     | Simple token transfer            |
| AI Call              | 100        | 0.000001    | AI model inference call          |
| Contract Deploy      | 100,000    | 0.001       | Deploy smart contract            |
| Contract Call        | 10,000     | 0.0001      | Execute contract method          |
| Governance           | 10,000     | 0.0001      | Governance proposal or vote      |
| Validator Operations | 10,000     | 0.0001      | Stake, register, or update node  |

> *1 IPN = 100 000 000 µIPN*

### 1.3 Fee Validation

Each transaction’s fee is checked using deterministic validation logic during admission:

```rust
// Example (in PoAConsensus block proposal)
for tx in block.transactions {
    validate_fee(&tx, tx.fee, &fee_config)?;
}
