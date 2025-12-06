# IPPAN Fuzz Harness

This crate contains fuzz targets for critical IPPAN components.

## Targets

### `canonical_hash`
Fuzzes canonical JSON serialization and hashing to ensure:
- Deterministic output for same input
- No panics on malformed JSON structures
- Proper rejection of invalid encodings

### `rpc_body_limit`
Fuzzes RPC request body size limits around boundary conditions:
- Correct rejection of oversized bodies
- Proper handling of bodies at limit boundaries
- No memory exhaustion on large inputs

### `proof_parsing`
Fuzzes proof bundle parsing and validation:
- No panics on arbitrary byte sequences
- Proper error handling for malformed proofs
- Safe parsing of proof structures

## Running Locally

```bash
# Install cargo-fuzz if needed
cargo install cargo-fuzz

# Run a target for 60 seconds
cargo fuzz run canonical_hash -- -max_total_time=60

# Run with corpus from previous runs
cargo fuzz run canonical_hash

# Reproduce a crash
cargo fuzz run canonical_hash -- -artifact_prefix=./artifacts/ ./artifacts/crash-*
```

## CI Integration

- **Smoke tests**: Run on PR/push with short time limits (~60s per target)
- **Nightly tests**: Weekly schedule with longer runs (10-30 min per target)

