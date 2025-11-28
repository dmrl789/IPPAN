# IPPAN Localnet DLC Dataset Export Guide

This document describes how to run the IPPAN localnet in DLC (Deterministic Learning Consensus) mode and export training datasets for GBDT model training.

## Prerequisites

- **Docker Desktop**: Required for running the localnet stack
- **Python 3**: For running the dataset exporter script
- **Git LFS**: For tracking large dataset files

## Quick Start

### 1. Start Localnet

```powershell
# Start the full localnet stack (3 nodes + gateway + explorer)
docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local up -d

# Wait for nodes to become healthy (about 30-60 seconds)
docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local ps
```

### 2. Verify DLC Metrics

```powershell
# Check that metrics_available=true and validators exist
$r = Invoke-RestMethod -Uri "http://127.0.0.1:8080/status"
Write-Host "Metrics available: $($r.consensus.metrics_available)"
Write-Host "Validators: $(if ($r.consensus.validators) { ($r.consensus.validators | Measure-Object).Count } else { 0 })"
Write-Host "Round: $($r.consensus.round)"
```

**Required**: `metrics_available` must be `true` and `validators` must be non-empty before exporting.

### 3. Export Dataset

```powershell
# Export 10,000 samples (recommended for training)
python ai_training\export_localnet_dataset.py `
    --rpc http://127.0.0.1:8080 `
    --samples 10000 `
    --interval 0.1 `
    --out ai_assets\datasets\localnet\localnet_training.csv
```

**Note**: With `--interval 0.1`, 10,000 samples takes approximately 16-17 minutes.

## Configuration

### DLC Consensus Mode

The localnet is configured to use DLC consensus via environment variables in `docker-compose.full-stack.yaml`:

```yaml
environment:
  IPPAN_CONSENSUS_MODE: "DLC"
  IPPAN_ENABLE_DLC: "true"
  IPPAN_STATUS_METRICS_DRIFT: "1"
  IPPAN_STATUS_METRICS_DRIFT_MODE: "tiers"
  IPPAN_STATUS_METRICS_DRIFT_SEED: "123"
```

### DLC Configuration File

The DLC configuration is mounted from `config/dlc.localnet.toml`:

- **Consensus mode**: `mode = "DLC"`, `enable_dlc = true`
- **Validator bond**: `require_validator_bond = false` (disabled for localnet)
- **D-GBDT model**: Mounted from `crates/ai_registry/models/ippan_d_gbdt_v3.json`

### Model Mount Path

The D-GBDT model file is mounted in containers at:
```
/app/crates/ai_registry/models/ippan_d_gbdt_v3.json
```

This path is configured in `config/dlc.localnet.toml`:
```toml
[dgbdt.model]
path = "/app/crates/ai_registry/models/ippan_d_gbdt_v3.json"
```

## Troubleshooting

### Metrics Not Available

If `metrics_available=false`:

1. **Check DLC is running**: Look for "Starting DLC consensus mode" in logs
   ```powershell
   docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local logs ippan-node | Select-String "DLC"
   ```

2. **Check metrics drift is enabled**: Verify `IPPAN_STATUS_METRICS_DRIFT=1` in compose file

3. **Wait longer**: Metrics drift may take 30-60 seconds to populate after node startup

4. **Check model file**: Ensure D-GBDT model is mounted correctly
   ```powershell
   docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local exec ippan-node ls -la /app/crates/ai_registry/models/
   ```

### Empty Validators

If `validators` is empty:

1. **Check node health**: All nodes should be `healthy`
   ```powershell
   docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local ps
   ```

2. **Check consensus round**: Round should advance (not stuck at 1)
   ```powershell
   $r = Invoke-RestMethod -Uri "http://127.0.0.1:8080/status"; $r.consensus.round
   ```

3. **Restart nodes**: Sometimes a full restart helps
   ```powershell
   docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local down -v
   docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local up -d
   ```

### Export Fails

If the exporter script fails:

1. **Check RPC endpoint**: Verify node is accessible
   ```powershell
   Invoke-RestMethod -Uri "http://127.0.0.1:8080/health"
   ```

2. **Check Python dependencies**: Install required packages
   ```powershell
   pip install requests
   ```

3. **Check output directory**: Ensure `ai_assets/datasets/localnet/` exists
   ```powershell
   New-Item -ItemType Directory -Force -Path ai_assets\datasets\localnet
   ```

## Dataset Format

The exported CSV contains the following columns:

- `timestamp_utc`: ISO 8601 timestamp
- `round_id`: Consensus round ID
- `validator_id`: 64-character hex validator ID
- `uptime_ratio_7d`: Uptime percentage (scaled to 0-1,000,000)
- `validated_blocks_7d`: Blocks verified in last 7 days (scaled)
- `missed_blocks_7d`: Blocks missed in last 7 days (scaled)
- `avg_latency_ms`: Average latency in milliseconds (scaled)
- `slashing_events_90d`: Number of slashing events
- `stake_normalized`: Stake amount (normalized 0-1, scaled)
- `peer_reports_quality`: Peer quality reports (normalized 0-1, scaled)
- `fairness_score`: Computed fairness score (0.0-1.0)

All scaled values use a scale factor of 1,000,000 to avoid floating-point in the CSV.

## Git LFS

Dataset files are tracked via Git LFS. After exporting:

```powershell
# Check LFS status
git lfs status

# Stage and commit
git add ai_assets/datasets/localnet/localnet_training.csv
git commit -m "chore(ai): export localnet training dataset"
git push origin master
```

## Reproducibility

To reproduce the exact same dataset:

1. Use the same seed: `IPPAN_STATUS_METRICS_DRIFT_SEED=123` (default in compose file)
2. Use the same drift mode: `IPPAN_STATUS_METRICS_DRIFT_MODE=tiers` (default)
3. Export with the same parameters: `--samples`, `--interval`
4. Ensure nodes start from clean state: `docker compose down -v` before starting

## Performance Notes

- **Export speed**: With `--interval 0.1`, expect ~600 samples/minute
- **Dataset size**: 10,000 samples ≈ 1.8-2.0 MB (compressed via LFS)
- **Node resources**: Each node uses ~100-200 MB RAM, minimal CPU when idle
- **Network**: Nodes communicate via Docker bridge network (no external traffic)

## See Also

- `config/dlc.localnet.toml`: DLC configuration file
- `localnet/docker-compose.full-stack.yaml`: Docker Compose configuration
- `ai_training/export_localnet_dataset.py`: Dataset exporter script

