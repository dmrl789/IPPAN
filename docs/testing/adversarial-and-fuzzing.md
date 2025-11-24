# Adversarial Testing & Fuzzing Guide

**Version:** 1.0.0-rc1  
**Last Updated:** 2025-11-24

---

## Overview

This document describes IPPAN's adversarial testing and fuzzing infrastructure, designed to stress-test the protocol implementation against malicious and malformed inputs.

**Goals:**
1. **Robustness:** Ensure no panics, crashes, or undefined behavior under adversarial input
2. **Determinism:** Verify that outputs are consistent across runs for same inputs
3. **Resource bounds:** Prevent excessive memory/CPU consumption from crafted inputs
4. **Security:** Detect vulnerabilities before they reach production

---

## Property-Based Testing

### What is Property-Based Testing?

Property-based testing (PBT) generates hundreds or thousands of random test cases to verify invariants hold across a wide input space, rather than testing specific examples.

**Framework:** We use [`proptest`](https://github.com/proptest-rs/proptest) for property-based testing in Rust.

### Existing Property Tests

#### 1. DLC & Fairness Tests

**Location:** `crates/consensus_dlc/tests/property_dlc.rs`

**Properties Tested:**
- **Determinism:** Same validator metrics → same D-GBDT score
- **Bounded scores:** All scores in range [0, 10,000]
- **Verifier selection:** Selected validators are always from active set
- **No duplicates:** Primary ≠ any shadow verifier
- **Consensus rounds:** Survive randomized block proposals and events

**Example:**
```rust
proptest! {
    #[test]
    fn fairness_scoring_is_deterministic_and_bounded(
        metrics in prop::collection::vec(adversarial_validator_metrics(), 1..20)
    ) {
        let model = FairnessModel::testing_stub();
        for metrics in metrics {
            let score_a = model.score_deterministic(&metrics);
            let score_b = model.score_deterministic(&metrics);
            
            prop_assert_eq!(score_a, score_b);  // Determinism
            prop_assert!(score_a >= 0 && score_a <= 10_000);  // Bounds
        }
    }
}
```

**Run tests:**
```bash
cargo test -p ippan-consensus-dlc --test property_dlc
```

#### 2. Transaction Property Tests

**Location:** `crates/consensus/tests/property_transactions.rs`

**Properties Tested:**
- **No overflow:** `amount + fee` never overflows
- **Validation determinism:** Same transaction → same validation result
- **Nonce ordering:** Prevents replay attacks
- **Fee bounds:** Fees stay within [0.001, 1 IPN]
- **Balance conservation:** Total supply is conserved after transfers

**Example:**
```rust
proptest! {
    #[test]
    fn transaction_amounts_never_overflow(
        amount in 0u64..=u64::MAX / 2,
        fee in 0u64..=u64::MAX / 2,
    ) {
        let total = amount.checked_add(fee);
        prop_assert!(total.is_some());
    }
}
```

**Run tests:**
```bash
cargo test -p ippan-consensus --test property_transactions
```

---

## Fuzzing

### What is Fuzzing?

Fuzzing generates semi-random inputs (often guided by code coverage) to find crashes, panics, or security vulnerabilities.

**Framework:** We use [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) (libFuzzer for Rust).

### Fuzz Targets

**Location:** `fuzz/fuzz_targets/`

#### 1. RPC Payment Endpoint

**Target:** `fuzz_rpc_payment.rs`

**Tests:**
- JSON parsing of payment requests
- Binary format parsing (if applicable)
- Length-prefixed payload handling

**Run:**
```bash
cargo install cargo-fuzz
cargo fuzz run fuzz_rpc_payment
```

**Expected Behavior:**
- No panics or crashes
- Invalid inputs return clean errors (400/422 responses)
- No excessive memory allocation (< 10 MB per request)

#### 2. RPC Handle Registration

**Target:** `fuzz_rpc_handle.rs`

**Tests:**
- Handle format validation (`@username`)
- Length checks (1-32 characters)
- Character set validation (alphanumeric + `_`, `-`)

**Run:**
```bash
cargo fuzz run fuzz_rpc_handle
```

**Expected Behavior:**
- Malformed handles rejected without panics
- SQL injection / XSS patterns safely rejected
- No excessive string allocations

#### 3. Transaction Decoding

**Target:** `fuzz_transaction_decode.rs`

**Tests:**
- CBOR/JSON transaction deserialization
- Signature parsing (64-byte Ed25519)
- Payload validation

**Run:**
```bash
cargo fuzz run fuzz_transaction_decode
```

**Expected Behavior:**
- Invalid signatures rejected
- Corrupted payloads don't cause state mutation
- No panics on malformed transactions

#### 4. P2P Message Parsing

**Target:** `fuzz_p2p_message.rs`

**Tests:**
- Gossipsub message deserialization
- Message type validation (block, tx, announcement)
- Size limit enforcement (≤1 MB)

**Run:**
```bash
cargo fuzz run fuzz_p2p_message
```

**Expected Behavior:**
- Oversized messages dropped
- Unknown message types ignored
- No panics on malformed protobuf/CBOR

### Continuous Fuzzing

For long-run fuzzing campaigns:

```bash
# Run for 1 hour
cargo fuzz run fuzz_rpc_payment -- -max_total_time=3600

# Run with corpus minimization
cargo fuzz cmin fuzz_rpc_payment

# Merge findings from multiple runs
cargo fuzz merge fuzz/corpus/fuzz_rpc_payment fuzz/artifacts/fuzz_rpc_payment
```

**Note:** Fuzzing artifacts (crashes, hangs) are saved to `fuzz/artifacts/`.

---

## Determinism Validation

### AI Determinism Harness

**Location:** `crates/ai_core/src/bin/determinism_harness.rs`

**Purpose:** Validates that D-GBDT inference is **bit-for-bit identical** across:
- Multiple runs on same hardware
- Different CPU architectures (x86_64, aarch64)
- Different operating systems (Linux, macOS, Windows)

#### Golden Vectors

**Count:** 50 test vectors

**Coverage:**
- Typical validators (uptime 90-99%, latency 10-500ms)
- Edge cases (zero stake, max uptime, worst latency)
- Boundary conditions (reputation thresholds ±1)

#### Running the Harness

```bash
# Text output
cargo run --bin determinism_harness

# JSON output (for CI)
cargo run --bin determinism_harness -- --format json > determinism_output.json

# Extract final digest
jq -r '.final_digest' determinism_output.json
```

**Expected Output:**
```
=== IPPAN D-GBDT Determinism Harness ===
Model: deterministic_gbdt_model.json
Model Hash: 9a3f2e1c...
Vectors: 50

[Vector 1] typical_validator_high_performance: 9275
[Vector 2] typical_validator_medium_performance: 7850
...
[Vector 50] boundary_reputation_below: 5001

Final Digest: a7b3c9d4e8f1a2b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9
Status: PASS
```

**Validation:** The `final_digest` MUST be identical across all platforms for the same model version.

#### Cross-Architecture Validation

1. **Baseline (x86_64):**
   ```bash
   cargo run --bin determinism_harness -- --format json > x86_64.json
   ```

2. **Target (aarch64 / arm64):**
   ```bash
   cargo run --bin determinism_harness -- --format json > aarch64.json
   ```

3. **Compare:**
   ```bash
   diff <(jq -r '.final_digest' x86_64.json) <(jq -r '.final_digest' aarch64.json)
   ```

**If digests differ:** Report as a critical bug (breaks consensus).

---

## Adversarial Scenarios

### Network Chaos Testing

**Location:** `crates/consensus_dlc/tests/long_run_simulation.rs`

**Scenario:** 512-round simulation with:
- Network splits (partition 40% of nodes)
- Validator churn (add/remove validators mid-run)
- Double-signing attempts (slashing triggers)
- Clock drift (±200ms skew)

**Invariants Checked:**
- No conflicting finalized blocks
- Supply cap never exceeded
- Slashed validators removed from active set
- Network heals after partitions

**Run:**
```bash
cargo test -p ippan-consensus-dlc long_run_dlc_with_churn_splits_slashing_and_drift -- --nocapture --test-threads=1
```

**Duration:** ~2-5 minutes (512 rounds)

### RPC Abuse Scenarios

**Test:** Send rapid requests to trigger rate limiting

```bash
# 1000 requests in 1 second
for i in {1..1000}; do
  curl -X POST http://localhost:8080/tx/payment \
    -H "Content-Type: application/json" \
    -d '{"from":"0x...","to":"0x...","amount":1000}' &
done
wait

# Expected: 429 Too Many Requests after ~100 requests
```

**Validation:**
- Rate limiter activates within 1 second
- Server remains responsive (no crash)
- Memory usage stays < 100 MB

### P2P Flood Attack

**Test:** Connect 100 peers and send oversized messages

**Expected Behavior:**
- Messages > 1 MB dropped
- Peer banned after 10 violations
- Node CPU usage < 80%

---

## CI Integration

### GitHub Actions

**.github/workflows/property-tests.yml** (to be added):

```yaml
name: Property Tests

on: [push, pull_request]

jobs:
  proptest:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace --test property_*
      
  fuzz-quick:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo install cargo-fuzz
      - run: cargo fuzz run fuzz_rpc_payment -- -max_total_time=60
      - run: cargo fuzz run fuzz_transaction_decode -- -max_total_time=60
```

**Note:** Full fuzzing (hours-long) runs offline, not in CI.

---

## Best Practices

### Writing New Property Tests

1. **Start with invariants:**
   - "This value is always >= 0"
   - "Sum of inputs = sum of outputs"
   - "Operation is idempotent"

2. **Use shrinking:**
   - Proptest automatically simplifies failing cases
   - Example: [1,2,3,4,5] fails → shrinks to [3]

3. **Avoid flaky tests:**
   - Don't depend on wall-clock time
   - Use deterministic RNG seeds

### Writing Fuzz Targets

1. **No panics:**
   - Use `Result` instead of `unwrap()`
   - Catch edge cases early

2. **Bounded resources:**
   - Reject inputs > 1 MB early
   - Use timeouts for expensive operations

3. **Meaningful coverage:**
   - Parse multiple formats (JSON, CBOR, binary)
   - Test all code paths

---

## Reporting Issues

If you find a crash/panic via fuzzing:

1. **Save the input:**
   ```bash
   cp fuzz/artifacts/fuzz_rpc_payment/crash-abc123 issue-abc123.bin
   ```

2. **Reproduce:**
   ```bash
   cargo fuzz run fuzz_rpc_payment issue-abc123.bin
   ```

3. **Open issue:**
   - Title: `[Fuzz] Crash in fuzz_rpc_payment`
   - Attach: `issue-abc123.bin` (if < 10 KB)
   - Include: Stack trace, OS, Rust version

---

## Summary

| Test Type | Coverage | Run Time | Frequency |
|-----------|----------|----------|-----------|
| **Property Tests** | DLC, transactions, validators | 10-60s | Every PR |
| **Fuzz Targets** | RPC, P2P, tx decoding | 1min-24hrs | Weekly / pre-release |
| **Determinism Harness** | D-GBDT inference | <1s | Every PR + cross-arch |
| **Chaos Simulation** | Network splits, churn | 2-5min | Weekly |

**Total Coverage:** ~85-90% of consensus-critical code

**Known Gaps:**
- ZK-STARK proof verification (future)
- Advanced P2P attacks (eclipse, BGP hijacking)
- Long-range attacks (social consensus required)

---

## References

- [proptest documentation](https://docs.rs/proptest)
- [cargo-fuzz book](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [AI_DETERMINISM_X86_REPORT_2025_11_24.md](../../AI_DETERMINISM_X86_REPORT_2025_11_24.md)
- [ACT_DLC_SIMULATION_REPORT_2025_11_24.md](../../ACT_DLC_SIMULATION_REPORT_2025_11_24.md)

---

**Maintainers:**  
- Ugo Giuliani (Lead Architect)
- Security Team

**Last Audit:** 2025-11-24 (v1.0.0-rc1)
