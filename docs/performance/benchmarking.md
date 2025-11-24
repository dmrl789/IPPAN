# IPPAN Benchmarking Guide

This document explains how to run the Phase 6 performance harness, compare results across changes, and find archived reports.

## What the suite measures

- **Mempool transaction validation throughput** – deterministic batch submission of 512 synthetic transactions exercising signature verification, fee policy, and broadcast queue logic.
- **Consensus round execution** – a full round with 64 validators, economic reward calculation, ledger settlement, and fee recycling.
- **Deterministic GBDT scoring** – AI scoring for 256 validators across 8 consecutive rounds using the fixed-point inference engine.
- **HashTimer ordering** – sorting and stable-priority ordering of 256 HashTimer samples to emulate block/transaction scheduling costs.

All microbenchmarks use seeded RNGs, purely in-memory data, and Criterion for reproducible statistics.

## Prerequisites

- Rust toolchain installed (`cargo`, `rustc`).
- Reasonable CPU time; the full suite takes ~3–4 minutes on an 8-core laptop because the mempool batch is intentionally heavy to stress signature checks.
- Optional: `gnuplot` for Criterion plots (Plotters backend is used automatically if missing).

## Running the suite

From the repository root:

```bash
./scripts/run-benchmarks.sh
```

The script will:

1. Build each bench target in release mode.
2. Run Criterion with `--warm-up-time 0.5s` and `--measurement-time 2s` (extended automatically for slower benches).
3. Print concise `time` / `thrpt` summaries to stdout.
4. Write the full command output to `benchmarks/reports/benchmarks_<UTC_TIMESTAMP>.log`.

To focus on a single component you can still call `cargo bench -p <crate> --bench <name>` directly, but prefer the script so reports stay uniform.

## Comparing results

1. Run the script on `master` (baseline) and on your feature branch.
2. Diff the latest log files in `benchmarks/reports/`. Example:

   ```bash
   diff benchmarks/reports/benchmarks_20251124T102612Z.log benchmarks/reports/benchmarks_20251126T083015Z.log
   ```

3. Pay attention to the Criterion mean/median lines and note any large (>5%) regressions.
4. Include the relevant summary lines in PR descriptions when optimizations or regressions are expected.

## Locating historical reports

- All raw outputs live under `benchmarks/reports/`.
- The most recent consolidated human-readable summary is published under `docs/performance/PERFORMANCE_REPORT_<date>.md`. Link to it from issues/PRs when referencing benchmark data.

## Troubleshooting

- **“Unable to complete 100 samples” warnings** – Criterion increases runtime automatically; no action needed unless runs become excessively long.
- **Benchmark panic** – inspect the matching `.log` file for stack traces, fix the underlying code, and rerun the script (old logs are kept for auditing).
- **Missing gnuplot** – install `gnuplot` if you need PNG output; otherwise Plotters’ SVG output is used.

By keeping the dataset, RNG seeds, and script constant, any change to bench results directly reflects code changes, which makes regressions easy to isolate.
