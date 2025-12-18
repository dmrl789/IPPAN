---
# IPPAN Time — Monotonic TimeState

## Why this matters
IPPAN time must be monotonic (never move backwards). A prior implementation split global time state across multiple mutexes with inconsistent lock ordering, which could produce racey snapshots under parallel execution and carried a real deadlock risk.

## What changed
- Consolidated time state into a single mutex (TimeState).
- now_us() enforces monotonicity unconditionally (peer offset ingestion cannot decrease computed time).
- Intended “median offset” behavior is preserved, but applied without allowing backwards jumps.

## Canary verification (on-node)
1) Service health:
   sudo systemctl status ippan-node --no-pager
   # Note: in the default devnet config, RPC listens on :8080 (P2P is :9000).
   curl -fsS http://127.0.0.1:8080/status | jq .

2) Peers:
   curl -fsS http://127.0.0.1:8080/peers | jq .

3) Monotonic time check:
   for i in $(seq 1 20); do
     curl -fsS http://127.0.0.1:8080/time | jq -r '.time_us'
     sleep 0.2
   done

   Expectation: values should never decrease.

## Rollback (if needed)
- Stop service: sudo systemctl stop ippan-node
- Restore previous binary: sudo cp -a /usr/local/bin/ippan-node.bak.<TIMESTAMP> /usr/local/bin/ippan-node
- Start service: sudo systemctl start ippan-node
- Re-run the health + /time checks.

## Health contract (stable checks for ops/CI)

### /status (RPC)
Fields relied on by ops tooling:
- `status`: string, expected `"ok"`
- `peer_count`: integer (devnet expected 4)
- `version`: string (semantic version)
- `build_sha`: string git commit (for drift detection; may be `"unknown"` if build metadata not embedded)

### /peers (RPC)
- JSON array of peer addresses; devnet expected `length == 4`

### /time (RPC)
- `time_us`: integer microseconds
- Must be monotonic over a short sample window (no decreases across successive samples)

## Devnet dataset export (D-GBDT telemetry)
- Exporter script: `/root/IPPAN/ai_training/export_localnet_dataset.py`
- Wrapper: `/usr/local/lib/ippan/export-dataset.sh`
- Output dir: `/var/lib/ippan/ai_datasets`
- File pattern: `devnet_dataset_*.csv.gz`
- Timer: `ippan-export-dataset.timer` (`OnCalendar=00,06,12,18:15 UTC; RandomizedDelaySec=600; Persistent=true`)
- Retention: `MAX_FILES=200; MAX_DIR_MB=2048`
- Lock: `/var/lock/ippan-export-dataset.lock` (`flock`)
- Enabled marker: `/etc/ippan/markers/dataset_export_enabled` (controls `/status.dataset_export.enabled`)
- Canonical build repo: `/opt/ippan` (build as `ippan-devnet`, not `root`)

Manual run:
  sudo systemctl start ippan-export-dataset.service
  sudo journalctl -u ippan-export-dataset.service -n 120 --no-pager
  ls -lh /var/lib/ippan/ai_datasets | tail -n 10

Troubleshooting:
  sudo systemctl status ippan-export-dataset.timer --no-pager
  sudo systemctl status ippan-export-dataset.service --no-pager
  sudo journalctl -u ippan-export-dataset.service -n 200 --no-pager
  python3 -c "import requests; print('requests ok')"

Related:
- Network baseline checks (latency/loss/throughput, bounded): see `docs/ops/network-test.md`

Operator “dataset freshness” checks:
- HTTP-only (preferred): `/status` includes `dataset_export = { enabled, last_ts_utc, last_age_seconds }`.
- SSH loop (quick spot-check):
    for ip in 5.223.51.238 188.245.97.41 135.181.145.174 178.156.219.107; do
      echo "=== $ip dataset freshness ==="
      ssh root@$ip "ls -1t /var/lib/ippan/ai_datasets/devnet_dataset_*.csv.gz 2>/dev/null | head -n 1 || true"
      ssh root@$ip "stat -c '%y %s %n' /var/lib/ippan/ai_datasets/devnet_dataset_*.csv.gz 2>/dev/null | tail -n 1 || true"
    done

## Policy decision (D-GBDT datasets + training + promotion)

This is the **operator/auditor-facing** intent for how devnet datasets are used.

- **Data source**: devnet exporter output on nodes: `/var/lib/ippan/ai_datasets/devnet_dataset_*.csv.gz`
- **Training cadence**: at most **once per 2 weeks**, or immediately after a material incident (time drift, peer collapse, sustained latency regression) that warrants re-weighting fairness.
- **Versioning**:
  - Dataset: record the exact filename set + UTC export timestamps; compute and record a content hash (BLAKE3) over the dataset bytes used for training.
  - Model: name as `ippan_d_gbdt_devnet_vN.json`; record its hash and training metadata in `docs/ai/MODEL_REGISTRY.md`.
- **Promotion rule (devnet → ship)**: a new model is promoted only if it passes:
  - Offline metrics thresholds on a held-out slice (no regression beyond defined tolerance),
  - Determinism checks (canonical JSON + stable hash verification),
  - Shadow-scoring vs current model on devnet for ≥ 24h with no safety regressions.
- **Retention**:
  - Datasets: keep at least **90 days** (or the exporter’s cap, whichever is larger).
  - Models: keep all shipped models + their training metadata indefinitely (audit trail), with an explicit “active” pointer via `config/dlc.toml` expected hash.
---


