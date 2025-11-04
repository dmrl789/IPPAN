# IPPAN Time

## Overview
- Supplies deterministic network time synchronization and HashTimer utilities.
- Ensures consensus participants share a bounded, monotonic microsecond clock.
- Supports optional peer time sync loops for distributed deployments.

## Key Modules
- `hashtimer`: generate, sign, and verify HashTimer payloads linked to consensus windows.
- `ippan_time`: core time service with median drift correction and monotonic guarantees.
- `sync`: optional async service to exchange time samples with peers.

## Integration Notes
- Call `init` during node bootstrap and `ingest_sample` as time reports arrive.
- Use `HashTimer` helpers for transaction timestamps and consensus round scheduling.
- Monitor `status` output to ensure drift stays within the configured bounds.
