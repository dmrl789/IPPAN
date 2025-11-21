# Comprehensive Testing – Phase 2 (Adversarial + Property Tests)

Phase 2 extends the deterministic coverage from Phase 1 with property-based fuzzing and adversarial request paths across consensus, RPC, and networking.

## DLC consensus property tests
- **Randomized validator metrics:** `crates/consensus_dlc/tests/property_dlc.rs` generates bounded validator metrics and asserts deterministic fairness scores remain within `[0, 10_000]` while avoiding panics.
- **Verifier selection stability:** property cases ensure randomly sized validator sets always produce selections drawn from the active set with no duplicates or out-of-range scores.
- **Randomized round sequences:** short randomized event traces insert optional blocks and advance consensus rounds, asserting non-negative reputation, bounded emission supply, and bonded stake persistence even when inputs fluctuate.

Run the suite:

```bash
cargo test -p ippan-consensus-dlc --tests -- --nocapture
```

## RPC adversarial tests
- **Malformed JSON and wrong methods:** `/tx/payment` returns clean 4xx responses for truncated JSON bodies or unsupported HTTP verbs without mutating counters.
- **Handle validation hardening:** oversized handles are rejected with structured errors and do not enter the registry.

Run the suite:

```bash
cargo test -p ippan-rpc -- --nocapture
```

## Network/DHT robustness tests
- **Malformed message decoding:** invalid `NetworkMessage` payloads surface serde errors instead of panics.
- **Peer churn and limits:** peer count enforcement resists over-capacity additions and cleans up metadata after removals.
- **Bad peer inputs:** empty or whitespace-only peer addresses are rejected without altering peer tables.

Run the suite:

```bash
cargo test -p ippan-p2p -- --nocapture
```

## Known limitations
- Property rounds are intentionally bounded (≤20 iterations) to keep CI fast; long-haul fuzzing remains a Phase 3 objective.
- Network robustness uses in-process harnesses only; no live multi-node soak tests yet.
