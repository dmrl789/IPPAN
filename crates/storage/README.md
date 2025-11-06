# IPPAN Storage

## Overview
- Abstracts persistence for blocks, accounts, L2 networks, and validator telemetry.
- Defines the `Storage` trait plus concrete Sled and in-memory implementations.
- Keeps serialization deterministic to support reproducible consensus and AI pipelines.

## Key Modules
- `lib.rs`: `Storage` trait, `SledStorage`, and `MemoryStorage` implementations.
- Chain state helpers for emissions, validator telemetry, and L2 artifacts.
- Error types aligned with database and serialization failure modes.

## Integration Notes
- Use `SledStorage::new` for production deployments; invoke `initialize` to seed genesis state.
- Swap in `MemoryStorage` during testing to avoid disk I/O while exercising the same trait surface.
- Persist validator telemetry and chain state updates before finalizing consensus rounds.
