# AI Model Governance for IPPAN

This document describes the governance process for registering, approving, and activating AI models on the IPPAN blockchain.  
It merges the detailed operational design (proposal, activation, and voting) with the technical governance rules used in the mainnet specification.

---

## 1. Overview

IPPAN employs **on-chain governance** to manage deterministic AI models used in the Layer-1 consensus reputation scoring and validation system.  
This process ensures updates remain:

- **Deterministic** — integer-only, reproducible, and consensus-safe  
- **Verifiable** — cryptographically signed and hash-pinned  
- **Transparent** — publicly auditable and open to community review  
- **Scheduled** — activated at a defined round, with clear version control  

Governance guarantees that no single entity can modify validator reputation logic or model parameters unilaterally.

---

## 2. Model Registry and Lifecycle

The on-chain `ModelRegistry` tracks all AI models used by the consensus layer.  
Each entry evolves through deterministic states:

| Status | Description |
|--------|-------------|
| **Proposed** | Submitted for governance review and validation |
| **Approved** | Approved through vote, pending activation |
| **Active** | Live in consensus; used for validator reputation |
| **Deprecated** | Superseded but retained for historical replay |
| **Revoked** | Disabled through emergency or security vote |

Each transition is authenticated by **Ed25519 signatures** and timestamped by **HashTimer rounds**.

---

## 3. Model Requirements

### 3.1 Canonical Format

Models must use deterministic JSON with no floating-point numbers and an explicit version header:

```json
{
  "version": 1,
  "feature_count": 8,
  "bias": 100,
  "scale": 10000,
  "trees": [...],
  "metadata": {
    "name": "reputation_v1",
    "description": "Validator reputation model",
    "training_data": "historical_validator_performance",
    "features": [
      "block_production_rate",
      "avg_block_size",
      "uptime",
      "network_latency",
      "validation_accuracy",
      "stake",
      "slashing_events",
      "last_activity"
    ],
    "performance_metrics": {
      "accuracy": 0.95,
      "precision": 0.93,
      "recall": 0.91
    }
  }
}
