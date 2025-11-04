# IPPAN Validator Resolution

## Overview
- Resolves validator identifiers to public keys, handles, and registry aliases.
- Bridges Layer 1 anchor proofs with Layer 2 handle registries for human-readable identities.
- Supports deterministic lookups leveraged by consensus, wallet, and governance flows.

## Key Modules
- `resolver`: high-level service that orchestrates lookup strategies.
- `errors`: distinct error types for missing anchors, stale registry entries, or invalid mappings.
- `types`: shared structs for validator records and resolution outcomes.
- `lib.rs`: crate entry point exposing resolver builders and helper traits.

## Integration Notes
- Combine with `ippan_l1_handle_anchors` and `ippan_l2_handle_registry` when wiring endpoints.
- Cache resolver results where possible to reduce repeated storage reads.
- Surface resolution errors clearly to operators to aid identity recovery workflows.
