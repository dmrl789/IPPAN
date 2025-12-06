# IPPAN Fuzz Testing

## Overview

The fuzz harness (`crates/fuzz`) provides property-based fuzzing for critical IPPAN components to catch:
- Memory safety issues
- Panics on malformed input
- Non-deterministic behavior
- Resource exhaustion
- Parsing vulnerabilities

## Fuzz Targets

### `canonical_hash`
Fuzzes canonical JSON serialization and hashing:
- **What it tests**: Deterministic JSON canonicalization, Blake3 hashing
- **Key properties**: Same input → same hash; no panics on arbitrary JSON
- **Location**: `crates/fuzz/fuzz_targets/canonical_hash.rs`

### `rpc_body_limit`
Fuzzes RPC request body size limit enforcement:
- **What it tests**: 64 KiB body limit, boundary conditions
- **Key properties**: Correct rejection of oversized bodies; no memory exhaustion
- **Location**: `crates/fuzz/fuzz_targets/rpc_body_limit.rs`

### `proof_parsing`
Fuzzes proof bundle parsing and validation:
- **What it tests**: Confidential transaction proof parsing, base64 decoding
- **Key properties**: No panics on arbitrary bytes; proper error handling
- **Location**: `crates/fuzz/fuzz_targets/proof_parsing.rs`

## Running Locally

### Prerequisites

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# Ensure you have nightly Rust (cargo-fuzz requires it)
rustup toolchain install nightly
```

### Smoke Tests (Quick Validation)

Run each target for 60 seconds:

```bash
cd crates/fuzz

# Run canonical_hash for 60 seconds
cargo fuzz run canonical_hash -- -max_total_time=60

# Run rpc_body_limit for 60 seconds
cargo fuzz run rpc_body_limit -- -max_total_time=60

# Run proof_parsing for 60 seconds
cargo fuzz run proof_parsing -- -max_total_time=60
```

### Long-Run Tests

Run targets for extended periods (e.g., 15 minutes):

```bash
cd crates/fuzz

# Run for 15 minutes (900 seconds)
cargo fuzz run canonical_hash -- -max_total_time=900

# Or use timeout command
timeout 900 cargo fuzz run canonical_hash
```

### Reproducing Crashes

If a crash is found:

```bash
cd crates/fuzz

# Reproduce a specific crash file
cargo fuzz run canonical_hash -- -artifact_prefix=./artifacts/ ./artifacts/crash-*

# Or run with the crash input directly
cargo fuzz run canonical_hash < crash_input.bin
```

## CI Integration

### Smoke Tests (PR/Push)

- **Workflow**: `.github/workflows/fuzz-smoke.yml`
- **Trigger**: Pull requests and pushes to master/main
- **Duration**: 60 seconds per target
- **Artifacts**: Crash inputs uploaded on failure (7-day retention)

### Nightly Tests (Long-Run)

- **Workflow**: `.github/workflows/fuzz-nightly.yml`
- **Trigger**: 
  - Weekly schedule (Sundays 02:00 UTC)
  - Manual via `workflow_dispatch` (configurable minutes)
- **Duration**: 15 minutes per target (default) or custom
- **Artifacts**: Corpus and crashes uploaded always (21-day retention)

## Reading Artifacts

### Location

After a workflow run:
1. Go to **Actions** → select the workflow run
2. Scroll to **Artifacts** section
3. Download `fuzz-smoke-crashes-<run_id>` or `fuzz-nightly-<run_id>`

### Crash Files

Crash files are binary inputs that trigger bugs. To analyze:

```bash
# View as hex
xxd crash-*

# Try to decode as UTF-8
cat crash-* | strings

# Reproduce locally
cargo fuzz run <target> -- -artifact_prefix=./artifacts/ ./artifacts/crash-*
```

### Corpus

The corpus contains interesting inputs discovered during fuzzing. It's stored in:
- `crates/fuzz/fuzz/corpus/<target>/`

The corpus helps fuzzing find more bugs by starting from known-good inputs.

## Troubleshooting

**cargo-fuzz not found:**
```bash
cargo install cargo-fuzz
```

**Nightly toolchain required:**
```bash
rustup toolchain install nightly
rustup default nightly  # or use +nightly in commands
```

**Out of memory:**
- Reduce `-max_total_time`
- Check input size caps in fuzz targets (should be capped at 1-2 MB)

**Targets don't compile:**
- Ensure all workspace dependencies are built: `cargo build --workspace`
- Check that fuzz crate is in workspace `Cargo.toml`

## Adding New Targets

1. Create a new file in `crates/fuzz/fuzz_targets/`
2. Add `[[bin]]` entry to `crates/fuzz/Cargo.toml`
3. Follow existing patterns:
   - Cap input size to prevent OOM
   - Use `Result` types, never panic
   - Test boundary conditions
4. Update this README

## Best Practices

- **Never panic**: Fuzz targets should return gracefully on all inputs
- **Cap sizes**: Prevent unbounded memory allocation
- **Test determinism**: Same input should produce same output
- **Boundary testing**: Test limits, edge cases, and off-by-one errors
- **No network calls**: Fuzz targets must be pure functions

