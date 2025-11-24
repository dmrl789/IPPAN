# IPPAN Performance Report — 2025-11-24

**Command:** `./scripts/run-benchmarks.sh`  
**Report artifact:** `benchmarks/reports/benchmarks_20251124T102612Z.log`

## Test environment

- **Host:** GitHub Codespaces / Ubuntu 24.04 container
- **CPU:** Intel(R) Xeon(R) Processor (4 vCPU, x86_64)
- **Rust:** 1.91.1 (stable) – release profile benches
- **gnuplot:** not installed (Plotters backend used)

## Results summary

| Target | Workload | Mean time | Throughput | Notes |
| --- | --- | --- | --- | --- |
| `ippan-mempool::tx_validation` | Validate & admit 512 signed transactions | **1.020 s** per batch | **~502 tx/s** | Signature verification + nonce ordering dominate. Criterion required extended measurement window. |
| `ippan-consensus::round_processing` | Execute 1 round w/ 64 validators | **~101.6 µs** | **~630k validator events/s** | Includes emission calc, payout ledger settlement, and fee recycling. |
| `ippan-ai-core::dgbdt_scoring` | Score 256 validators across 8 rounds | **~318 µs** | **~6.44 M feature evals/s** | Deterministic fixed-point D-GBDT inference w/ seeded telemetry. |
| `ippan-time::hashtimer_ordering` | Sort 256 HashTimers | **~6.48 µs** | **~39.5 M items/s** | Secondary “stable priority queue” variant measures at ~6.56 µs. |

## Interpretation

- **End-to-end transaction capacity:** 502 tx/s per core for pure validation places an upper bound on single-node throughput. With four workers this extrapolates to ~2,000 tx/s before networking/storage effects. Further gains will come from parallel signature verification.
- **Round finalization latency:** 100 µs per DAG-Fair round suggests the economics path is not the bottleneck; even with 10k rounds/second the executor remains <60% utilized on one core.
- **AI scoring headroom:** 6.4M feature evaluations per second comfortably supports re-scoring thousands of validators every round while leaving time for telemetry updates.
- **HashTimer ordering margins:** Sub-7 µs ordering indicates HashTimer-based prioritization has negligible impact compared to networking and mempool time.

## Caveats & next steps

- Benchmarks avoid disk/network IO and therefore represent ideal CPU-only performance. Real deployments will see lower throughput once RocksDB/sled storage and libp2p gossip are included.
- The mempool bench currently executes sequentially; introducing rayon-based signature verification should provide a significant boost and will require an updated report.
- Keep the `benchmarks/reports/*.log` artifacts; when regressions are suspected, re-run the script on `master` and compare logs to quantify the delta.
