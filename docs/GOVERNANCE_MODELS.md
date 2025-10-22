# AI Model Governance for IPPAN

This document describes the governance process for registering and activating AI models on the IPPAN blockchain.

## Overview

IPPAN uses on-chain governance to manage AI models used in L1 consensus for reputation scoring. All models must be:

1. **Deterministic**: Integer-only GBDT models with reproducible outputs
2. **Verified**: Cryptographically signed by foundation or approved via governance
3. **Transparent**: Full model structure available for audit
4. **Scheduled**: Activated at a specific round to ensure network-wide synchronization

## Model Requirements

### Technical Requirements

- **Format**: JSON with GBDT tree structure
- **Arithmetic**: Integer-only (no floating point)
- **Features**: 6 input features (see Feature Specification below)
- **Output**: Scaled integer in range [0, 10000] representing reputation score
- **Size**: < 1MB serialized
- **Hash**: SHA-256 of canonical JSON representation

### Feature Specification

All models must accept exactly 6 features, each scaled to [0, 10000]:

0. **Proposal Rate**: `(blocks_proposed / rounds_active) * scale / max_blocks_proposed`
1. **Verification Rate**: `(blocks_verified / rounds_active) * scale / max_blocks_proposed`
2. **Latency Score**: `scale - (avg_latency_us * scale / max_latency_us)` (inverted, lower latency = higher score)
3. **Slash Penalty**: `scale - (slash_count * 1000)` (clamped to 0)
4. **Stake Weight**: `(stake * scale) / max_stake`
5. **Longevity**: `(age_rounds * scale) / max_age_rounds`

Scale = 10,000

## Governance Proposal Format

### JSON/YAML Proposal Template

```json
{
  "model_id": "reputation_v2",
  "version": 2,
  "model_hash": "0x1234...abcd",
  "model_url": "ipfs://Qm.../reputation_v2.json",
  "activation_round": 5000000,
  "signature_foundation": "0xabcd...1234",
  "proposer_pubkey": "0x5678...ef90",
  "rationale": "Improved accuracy with additional trees for edge cases. Tested on 1M rounds of historical data with 15% better prediction of validator uptime.",
  "threshold_bps": 6667
}
```

### Field Descriptions

- **model_id**: Unique identifier (semver recommended, e.g., `reputation_v2`)
- **version**: Integer version number
- **model_hash**: SHA-256 hash of model structure (hex or base64)
- **model_url**: Location for model download (IPFS preferred, https:// allowed)
- **activation_round**: Future round when model activates if approved
- **signature_foundation**: Ed25519 signature from proposer
- **proposer_pubkey**: Public key of proposer (foundation or governance multi-sig)
- **rationale**: Human-readable explanation with test results
- **threshold_bps**: Voting threshold in basis points (6667 = 66.67% supermajority)

## Proposal Lifecycle

```
┌──────────┐
│ Proposed │ 
└────┬─────┘
     │
     │ Vote Period (7 days / ~3M rounds)
     ├─> Rejected ──> End
     │
     ├─> Approved
     │
┌────▼────────┐
│  Approved   │
└────┬────────┘
     │
     │ Activation Round Reached
     │
┌────▼────────┐
│   Active    │
└────┬────────┘
     │
     │ New Model Activated
     │
┌────▼─────────┐
│  Deprecated  │
└──────────────┘
```

### States

1. **Proposed**: Submitted on-chain, voting in progress
2. **Approved**: Vote passed, waiting for activation round
3. **Active**: Currently used in consensus
4. **Deprecated**: Replaced but still valid for historical verification
5. **Revoked**: Emergency removal (requires governance override)

## Voting Process

### Eligibility

Only active validators with stake ≥ 1,000 IPN can vote.

### Voting Weight

Each validator's vote weight = `stake * reputation_score / 10000`

### Threshold

- **Standard**: 66.67% (6667 bps) of total voting weight
- **Emergency Revocation**: 75% (7500 bps)

### Voting Period

7 days from proposal submission (~3,024,000 rounds at 200ms/round)

## Signature Verification

### Message Format (for signing)

```
model_id || version || model_hash || model_url || activation_round
```

All fields concatenated as raw bytes (integers in big-endian).

### Signature Scheme

Ed25519 with foundation public key or governance multi-sig.

### Verification Command

```bash
# Using IPPAN CLI
ippan governance verify-proposal proposal.json

# Manual verification
echo -n "reputation_v2" | cat - <(echo -n "00000002") <(xxd -r -p <<< "1234...abcd") \
  <(echo -n "ipfs://Qm...") <(echo -n "0000000000500000") | \
  ippan-verify-sig --pubkey 0x5678...ef90 --sig 0xabcd...1234
```

## Model Submission Process

### 1. Develop and Test Model

```python
# Train GBDT with integer features
from sklearn.ensemble import GradientBoostingClassifier
# ... training code ...

# Export to integer-only format
export_to_ippan_json(model, "reputation_v2.json")
```

### 2. Compute Hash

```bash
# Canonical JSON (sorted keys, no whitespace)
jq -S -c . reputation_v2.json | sha256sum
```

### 3. Upload Model

```bash
# Upload to IPFS
ipfs add reputation_v2.json
# Returns: Qm.../reputation_v2.json
```

### 4. Create Proposal

```bash
ippan governance create-proposal \
  --model-id reputation_v2 \
  --version 2 \
  --hash 0x1234...abcd \
  --url ipfs://Qm.../reputation_v2.json \
  --activation-round 5000000 \
  --rationale "Improved accuracy..." \
  --key foundation.key
```

### 5. Submit On-Chain

```bash
ippan tx governance submit proposal.json
```

### 6. Vote

```bash
# Validators vote
ippan tx governance vote --proposal-id 42 --vote yes --key validator.key
```

## Activation

Once approved and activation round reached:

1. Validators download model from `model_url`
2. Verify hash matches `model_hash`
3. Load model into memory
4. Begin using for reputation scoring in round `activation_round + 1`

## Emergency Procedures

### Model Revocation

If critical bug discovered:

1. Emergency governance vote (75% threshold)
2. Immediate deactivation
3. Revert to previous active model or default scoring

```bash
ippan governance emergency-revoke --proposal-id 42 --reason "Integer overflow in tree 15"
```

### Rollback

Blockchain state rollback to last block before buggy model activation (requires consensus among validators).

## Testing Requirements

Before proposal, models must pass:

1. **Determinism Test**: Same inputs → same outputs on all platforms
2. **Range Test**: All outputs ∈ [0, 10000]
3. **No-Float Test**: No f32/f64 usage in evaluation code
4. **Hash Verification**: Computed hash matches declared hash
5. **Historical Simulation**: Run on ≥ 100k historical rounds, compare with current model

## Example Models

### Minimal Model (1 tree)

```json
{
  "metadata": {
    "model_id": "reputation_minimal",
    "version": 0,
    "model_type": "gbdt",
    "hash_sha256": "...",
    "feature_count": 6,
    "output_scale": 10000,
    "output_min": 0,
    "output_max": 10000
  },
  "model": {
    "trees": [
      {
        "nodes": [
          {"feature_index": 3, "threshold": 8000, "left": 1, "right": 2, "value": null},
          {"feature_index": 0, "threshold": 0, "left": 0, "right": 0, "value": 3000},
          {"feature_index": 0, "threshold": 0, "left": 0, "right": 0, "value": 8000}
        ]
      }
    ],
    "bias": 2000,
    "scale": 10000
  }
}
```

Logic: If slash_penalty > 8000, return 8000 + 2000 = 10000; else return 3000 + 2000 = 5000.

### Production Model (30 trees, depth 6)

See `models/reputation_v1.json` in repository.

## References

- [AI Core Implementation](../crates/ai_core/)
- [AI Registry Types](../crates/ai_registry/)
- [Consensus Integration](../crates/consensus/src/reputation.rs)
- [Emission Schedule](./FEES_AND_EMISSION.md)

## Changelog

- **2025-10-22**: Initial governance specification
