# AI Model Governance for IPPAN

This document describes the governance process for registering, approving, and activating AI models on the IPPAN blockchain.  
It merges the detailed operational design (proposal, activation, and voting) with the technical governance rules used in mainnet specification.

---

## 1. Overview

IPPAN employs **on-chain governance** to manage deterministic AI models used for L1 consensus reputation scoring.  
This ensures updates remain **transparent**, **verifiable**, and **democratically controlled**.

All approved models must be:

1. **Deterministic** — integer-only GBDT with reproducible results  
2. **Verified** — cryptographically signed and hash-pinned  
3. **Transparent** — publicly auditable structure  
4. **Scheduled** — activated at a defined future round  

---

## 2. Model Registry and States

The on-chain `ModelRegistry` maintains all models and their lifecycle:

| Status | Description |
|--------|-------------|
| **Proposed** | Submitted, voting in progress |
| **Approved** | Vote passed, waiting for activation |
| **Active** | Currently used in consensus |
| **Deprecated** | Replaced but valid for historical verification |
| **Revoked** | Emergency-disabled via governance |

---

## 3. Model Requirements

### 3.1 Format

Models must use canonical JSON with fields:

```json
{
  "version": 1,
  "feature_count": 6,
  "bias": 100,
  "scale": 10000,
  "trees": [...],
  "metadata": {...}
}
