# IPPAN RC Performance Baseline

This document captures a first-pass performance baseline for the RC build. The focus is to provide a reproducible harness (benchmarks + load generator) and a place to record measurements before any optimization work begins.

## Environment

- IPPAN version: v0.9.0-rc1 (commit _fill-in after run_)
- Machine: _e.g., 8 vCPU / 16 GB RAM, Ubuntu 22.04, x86_64_
- Node config: `config/local-node.toml` with `--dev` (single node) unless otherwise noted
- Notes: run `scripts/perf/run-rc-node-for-perf.sh` in one shell and `scripts/perf/run-rc-load-test.sh` in another on the same host.

## Micro-benchmarks

Benchmarks are implemented with Criterion and can be executed individually:

```bash
cargo bench -p ippan-time --bench hash_timer_bench
cargo bench -p ippan-ai-core --bench dgbdt_inference_bench
cargo bench -p ippan-storage --bench block_apply_bench
```

Record the latest numbers here after running on the target hardware:

- HashTimer `now_us()`: _ns/op_
- D-GBDT inference on `reputation_v1.json`: _µs per call_
- Memory storage block apply (64 tx synthetic block): _µs per apply_

## Load test

The `ippan-loadgen` tool issues payment RPCs against a running RC node.

Example invocation (defaults to 10k tx, concurrency 32):

```bash
scripts/perf/run-rc-load-test.sh
# or manually
cargo run -p ippan-loadgen --release -- \
  --rpc http://127.0.0.1:3000 \
  --tx-count 10000 \
  --concurrency 32 \
  --amount 1000 \
  --nonce-start 1
```

Capture results in this section after executing:

- Transactions: _N_
- Concurrency: _C_
- Observed TPS: _T_
- Mean latency: _L ms_ (p95: _P ms_)
- Notes: _CPU/memory observations, RPC error rates_

## Interpretation

- These numbers establish the RC baseline; they are not optimized targets.
- Keep the harness stable so future optimization passes can compare directly.
- If hardware, config, or topology changes, add a short note alongside the measurements.
