# Devnet Batch Ingest Runbook: 200k → 500k Offered TPS

This runbook validates **high-throughput ingestion** at 200k–500k offered TPS using the dedicated batch endpoint with **clean overload shedding (429-only)** and **truthful measurements**.

## Quick Reference

| Item | Value |
|------|-------|
| Endpoint | `POST /tx/submit_batch` (`application/octet-stream`) |
| Tool | `ippan-txload batch` |
| Script | `scripts/ops/ramp_batch_500k.sh` |
| Nodes | `api1.ippan.uk` (primary), `api2-4.ippan.uk` |
| RPC Port | `8080` |
| Service | `ippan-node` |

## Global Rules

1. **No consensus changes** (no DLC/round/validator-set/proposer edits)
2. **Batch lane must shed load with HTTP 429** (never timeouts/resets; no 500s under overload)
3. **/health + /status must stay fast** and must not share heavy middleware with batch lane
4. **One source of truth for batch env**: single systemd drop-in `70-batch-ingest.conf`
5. **Measurements must be "truthful"**:
   - `client_errors` must be 0 (or we stop)
   - `invalid` must be 0 (or we stop)
   - If `dropped_queue_full > 0`, that's a **client bottleneck** → fix txload knobs, not the node
6. **Run high TPS tests on api1 locally** (no tunnel)

---

## Step 0: Stop Drift — Verify Clean State

### Check git status

```bash
cd /mnt/c/Users/ugosa/Desktop/Cursor\ SOFTWARE/IPPAN
git status --porcelain
git log -n 3 --oneline
```

### Create dedicated branch (if needed)

```bash
git checkout -b ops/batch-200k-500k-runbook
```

### Gate

`git status --porcelain` should only show files in:
- `scripts/ops/*batch*`
- `docs/ops/*batch*`
- `crates/rpc/src/server.rs` (only if needed)
- `tools/txload/src/main.rs` (only if needed)

---

## Step 1: Confirm Real ENV VAR Names

**CRITICAL**: The server uses specific env var names. Using wrong names is the #1 cause of "we set 4096 but it still enforces 2000".

Run from repo root:

```bash
rg -n "IPPAN_BATCH_|ENABLE_BATCH|submit_batch" crates/rpc/src/server.rs | head -30
```

### Correct ENV VAR Names (from `crates/rpc/src/server.rs`)

| Purpose | Correct Name | ❌ Wrong Names |
|---------|-------------|----------------|
| Enable batch | `IPPAN_ENABLE_BATCH_SUBMIT` | |
| Max TX per batch | `IPPAN_BATCH_MAX_TX_PER_BATCH` | `IPPAN_BATCH_MAX_TXS` |
| Body limit bytes | `IPPAN_BATCH_BODY_LIMIT_BYTES` | `IPPAN_BATCH_MAX_BODY_BYTES` |
| Concurrency limit | `IPPAN_BATCH_CONCURRENCY_LIMIT` | |
| Queue capacity | `IPPAN_BATCH_QUEUE_CAPACITY` | |
| Mempool backpressure | `IPPAN_BATCH_BACKPRESSURE_MEMPOOL_SIZE` | |
| Decode workers | `IPPAN_BATCH_DECODE_WORKERS` | |

---

## Step 2: Install Authoritative Systemd Drop-In

We install **one** file: `/etc/systemd/system/ippan-node.service.d/70-batch-ingest.conf`

### 2.1 Install on all nodes

```bash
# From WSL, run the install script:
./scripts/ops/install_batch_ingest_dropin.sh api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk
```

Or manually on api1:

```bash
ssh root@api1.ippan.uk "set -e
mkdir -p /etc/systemd/system/ippan-node.service.d
cat > /etc/systemd/system/ippan-node.service.d/70-batch-ingest.conf <<'EOF'
[Service]
# Single authoritative batch ingest configuration
Environment=IPPAN_DEV_MODE=true
Environment=IPPAN_ENABLE_BATCH_SUBMIT=1

# CRITICAL: Use CORRECT env var names from crates/rpc/src/server.rs
Environment=IPPAN_BATCH_MAX_TX_PER_BATCH=4096
Environment=IPPAN_BATCH_BODY_LIMIT_BYTES=268435456
Environment=IPPAN_BATCH_CONCURRENCY_LIMIT=256
Environment=IPPAN_BATCH_QUEUE_CAPACITY=16384
Environment=IPPAN_BATCH_BACKPRESSURE_MEMPOOL_SIZE=100000

Environment=RUST_LOG=info,ippan_rpc=info
EOF

# Disable drift sources
cd /etc/systemd/system/ippan-node.service.d
for f in 15-dlc-config.conf 31-dlc-runtime.conf 99-dlc-config-path.conf 99-consensus-validators.conf 99-validator-set.conf 30-bootstrap.conf 99-bootstrap-override.conf; do
  [ -f \"\$f\" ] && mv -f \"\$f\" \"\$f.disabled\"
done

systemctl daemon-reload
systemctl restart ippan-node
sleep 2
systemctl is-active ippan-node
curl -sS -m 2 http://127.0.0.1:8080/health; echo
"
```

### 2.2 Verify no drift

```bash
./scripts/ops/check_batch_dropin_drift.sh api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk
```

Gate: Only `70-batch-ingest.conf` should define `IPPAN_BATCH_*` variables.

---

## Step 3: Ensure Server Limits Match Client

| Setting | Server (drop-in) | Client (txload) |
|---------|-----------------|-----------------|
| Max TX per batch | `IPPAN_BATCH_MAX_TX_PER_BATCH=4096` | `--batch-size 1024` |
| Body limit | `IPPAN_BATCH_BODY_LIMIT_BYTES=268435456` | (auto) |

**Rule**: `IPPAN_BATCH_MAX_TX_PER_BATCH` must be >= `--batch-size`

For 200k–500k offered, use `--batch-size 1024` (not 4096) to reduce decode/verify burst latency.

---

## Step 4: Build & Deploy

### 4.1 Build locally (WSL)

```bash
cd /mnt/c/Users/ugosa/Desktop/Cursor\ SOFTWARE/IPPAN
cargo build --release -p ippan-node
cargo build --release -p ippan-txload
ls -la target/release/ippan-node target/release/ippan-txload
```

### 4.2 Deploy to api1

```bash
# Deploy node binary
scp target/release/ippan-node root@api1.ippan.uk:/usr/local/bin/ippan-node.new
ssh root@api1.ippan.uk 'set -e
  systemctl stop ippan-node
  cp -a /usr/local/bin/ippan-node /usr/local/bin/ippan-node.bak.$(date -u +%Y%m%dT%H%M%SZ)
  install -m 0755 /usr/local/bin/ippan-node.new /usr/local/bin/ippan-node
  rm -f /usr/local/bin/ippan-node.new
  systemctl start ippan-node
  sleep 2
  systemctl is-active ippan-node
'

# Deploy txload
scp target/release/ippan-txload root@api1.ippan.uk:/usr/local/bin/ippan-txload
ssh root@api1.ippan.uk 'chmod +x /usr/local/bin/ippan-txload'
```

### 4.3 Verify deployment

```bash
ssh root@api1.ippan.uk 'curl -sS -m 2 http://127.0.0.1:8080/health; echo'
ssh root@api1.ippan.uk 'curl -sS -m 2 http://127.0.0.1:8080/status | head -c 500; echo'
ssh root@api1.ippan.uk '/usr/local/bin/ippan-txload batch --help | head -30'
```

---

## Step 5: Prepare Senders

For 200k–500k offered TPS, use **512+ senders** to avoid nonce contention.

### Generate senders (if needed)

```bash
ssh root@api1.ippan.uk 'mkdir -p /var/lib/ippan/out/senders'
/usr/local/bin/ippan-txload gen-senders --count 512 --out /var/lib/ippan/out/senders/senders.json
```

### Fund senders

Use existing funding scripts or manually fund each sender with enough balance for the test duration.

---

## Step 6: Run 200k → 500k Benchmark

### 6.1 Create run directory

```bash
ssh root@api1.ippan.uk "set -e
cd /var/lib/ippan
STAMP=\$(date -u +%Y%m%dT%H%M%SZ)
ROOT=/var/lib/ippan/out/batch_\$STAMP
mkdir -p \$ROOT
echo \"Created: \$ROOT\"
"
```

### 6.2 Run the ramp script

```bash
ssh root@api1.ippan.uk 'set -e
cd /var/lib/ippan

export RPC=http://127.0.0.1:8080
export TO_ADDR=<YOUR_RAW_TO_ADDRESS>
export NONCE_START=1
export SENDERS_FILE=/var/lib/ippan/out/senders/senders.json

# High-TPS tuned settings
export BATCH_SIZE=1024
export CONCURRENCY=256
export MAX_INFLIGHT=8192
export MAX_QUEUE=200000
export DRAIN_SECONDS=10
export STAGE_SECONDS=20

bash scripts/ops/ramp_batch_500k.sh
'
```

### 6.3 Manual staged run (alternative)

```bash
ssh root@api1.ippan.uk "set -e
cd /var/lib/ippan
ROOT=\$(ls -dt /var/lib/ippan/out/batch_* | head -n 1)
RPC=http://127.0.0.1:8080
TO_ADDR=<YOUR_RAW_TO_ADDRESS>
SENDERS_FILE=/var/lib/ippan/out/senders/senders.json
NONCE_START=1

run_stage() {
  TPS=\"\$1\"; SECS=\"\$2\"; LABEL=\"\$3\"
  LOG=\"\$ROOT/run_\${LABEL}_\${SECS}s.log\"
  SUMMARY=\"\$ROOT/summary_\${LABEL}.txt\"
  
  echo \"== stage offered=\${TPS} tps for \${SECS}s ==\"
  curl -sS -m 2 \$RPC/health; echo
  
  /usr/local/bin/ippan-txload batch \\
    --rpc \$RPC \\
    --tps \$TPS \\
    --seconds \$SECS \\
    --to \$TO_ADDR \\
    --senders-file \$SENDERS_FILE \\
    --nonce-start \$NONCE_START \\
    --batch-size 1024 \\
    --concurrency 256 \\
    --max-inflight 8192 \\
    --drain-seconds 10 \\
    2>&1 | tee \$LOG
  
  grep '^SUMMARY ' \$LOG | tail -1 > \$SUMMARY
  cat \$SUMMARY
}

run_stage 200000 20 200k
run_stage 300000 20 300k
run_stage 400000 20 400k
run_stage 500000 20 500k

echo 'DONE. Summaries:'
ls -1 \$ROOT/summary_*
"
```

---

## Step 7: Collect Results

```bash
ssh root@api1.ippan.uk "set -e
ROOT=\$(ls -dt /var/lib/ippan/out/batch_* | head -n 1)
echo \"ROOT=\$ROOT\"
for f in \$ROOT/summary_*.txt; do
  echo \"--- \$(basename \$f) ---\"
  cat \$f
done
echo '--- health ---'
curl -sS -m 2 http://127.0.0.1:8080/health; echo
echo '--- status ---'
curl -sS -m 2 http://127.0.0.1:8080/status | head -c 500; echo
"
```

### Run invariant check

```bash
./scripts/ops/devnet_invariant_check.sh api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk
```

---

## Interpreting SUMMARY Lines

Each stage produces:

```
SUMMARY offered_tps=... accepted_tps=... http_429=... invalid=... client_errors=... dropped_queue_full=...
```

### Decision Matrix

| Symptom | Diagnosis | Action |
|---------|-----------|--------|
| `http_429` high, `client_errors=0` | ✅ Good overload shedding | Server lane is the limit |
| `client_errors > 0` | ❌ Ingestion collapse | Fix server lane / admission path |
| `invalid > 0` | ❌ Nonce collisions or bad txs | Check sender nonce ranges |
| `dropped_queue_full > 0` | ⚠️ Client bottleneck | Increase `--max-inflight`, `--max-queue`, or senders |
| Accepted TPS plateaus at ~1k-10k | Normal capacity limit | Optimize verify+decode+enqueue path |

---

## Step 8: If Accepted TPS Plateaus Low

If offered is 200k–500k but accepted is ~1k–10k with clean 429s:
- That's not failure — it means **verify+decode+enqueue path** is the limiter.

Next optimizations (without touching consensus):
1. Pre-parse and verify in parallel (CPU bound)
2. Avoid per-tx allocations in batch decode
3. Use fixed-size worker pool + lock-free ring buffers
4. Batch enqueue into mempool with fewer locks

---

## Results Table Template

| Stage | Offered TPS | Accepted TPS | http_429 | invalid | client_errors | dropped_queue_full | Notes |
|------:|------------:|-------------:|---------:|--------:|--------------:|-------------------:|-------|
| 200k/20s | | | | | | | |
| 300k/20s | | | | | | | |
| 400k/20s | | | | | | | |
| 500k/20s | | | | | | | |

---

## Troubleshooting

### Wrong ENV VAR names
**Symptom**: "we set 4096 but it still enforces 2000"

**Fix**: Use `IPPAN_BATCH_MAX_TX_PER_BATCH`, not `IPPAN_BATCH_MAX_TXS`

### Multiple drop-ins defining batch vars
**Symptom**: Inconsistent behavior after restart

**Fix**: Run `./scripts/ops/check_batch_dropin_drift.sh` and disable extras

### Client queue full
**Symptom**: `dropped_queue_full > 0`

**Fix**: Increase `--max-inflight 16384` and `--max-queue 500000`

### Nonce collisions
**Symptom**: `invalid` spikes, accepted collapses

**Fix**: Increase sender count, ensure non-overlapping nonce ranges

