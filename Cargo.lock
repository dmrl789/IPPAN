# Fee Caps and Emission Schedule

This document specifies the IPPAN protocol's fee caps, emission schedule, and reward distribution logic.

---

## 🪙 Fee Caps

### Overview

IPPAN enforces **hard, deterministic fee caps** at the protocol level to ensure predictable costs and prevent market manipulation.  
Transactions exceeding the cap are rejected during both **mempool admission** and **block assembly**.

### Cap Values (µIPN)

| Transaction Type | Cap (µIPN) | Cap (IPN) | USD Equivalent* |
|------------------|------------|-----------|-----------------|
| Transfer         | 1,000      | 0.00001   | ~$0.0001        |
| AI Call          | 100        | 0.000001  | ~$0.00001       |
| Contract Deploy  | 100,000    | 0.001     | ~$0.01          |
| Contract Call    | 10,000     | 0.0001    | ~$0.001         |
| Governance       | 10,000     | 0.0001    | ~$0.001         |
| Validator Ops    | 10,000     | 0.0001    | ~$0.001         |

\*Assuming $10/IPN, illustrative only.

### Enforcement

```rust
// In mempool admission
if tx.fee > fee_cap_for_type(tx.type) {
    return Err(FeeError::FeeAboveCap);
}

// In block proposal
for tx in block.transactions {
    validate_fee(&tx, tx.fee, &fee_config)?;
}
