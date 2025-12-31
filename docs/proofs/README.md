# IPPAN Proof Bundles

This directory contains cryptographic proof bundles and attestations for IPPAN's determinism guarantees.

## Proof Bundle Index

| Bundle ID | Description | Date | SHA-256 | Status |
|-----------|-------------|------|---------|--------|
| 2B-B | Verifier Selection Determinism (4-host devnet) | 2025-12-31 | `7c3e81e1eb945c049c0bf78a0a699511746e0aa92a12f14a067cb9d49e6bd4d3` | ✅ PASS |

## Files

- `proof_2B-B_verifier-selection_dlc_2025-12-31.tar.gz` — Proof archive for verifier selection determinism
- `VERIFIER_SELECTION_DETERMINISM_ATTESTATION.md` — Detailed attestation document

## Verification

To verify a proof bundle:

```bash
# Check SHA-256
sha256sum docs/proofs/proof_2B-B_verifier-selection_dlc_2025-12-31.tar.gz
# Expected: 7c3e81e1eb945c049c0bf78a0a699511746e0aa92a12f14a067cb9d49e6bd4d3

# Extract and inspect
tar -tzf docs/proofs/proof_2B-B_verifier-selection_dlc_2025-12-31.tar.gz
tar -xzf docs/proofs/proof_2B-B_verifier-selection_dlc_2025-12-31.tar.gz
```

## Determinism Claim

> **IPPAN DLC verifier selection is multi-host deterministic across 4 devnet hosts when configured with the same validator set and seeded metrics. For the same `selection_round_id`, all hosts compute identical `selection_seed`, `round_primary_id`, and `selection_commitment`.**

See [`VERIFIER_SELECTION_DETERMINISM_ATTESTATION.md`](./VERIFIER_SELECTION_DETERMINISM_ATTESTATION.md) for full details.
