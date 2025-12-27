### Devnet batch ingest runbook (20k → 50k → 200k offered) + “429-only overload”

This runbook validates **high-throughput ingestion** against the dedicated batch endpoint:

- `POST /tx/submit_batch` (`application/octet-stream`, counted length-delimited frames)
- Load tool: `ippan-txload batch`
- Ramp scripts (run on **api1**): `scripts/ops/ramp_batch_{20k,50k,200k}.sh`

Success means **no client-side failures** under overload:

- **Expected**: many `http_429` responses at 50k/200k offered
- **Required**: `client_errors=0` and `/health` stays responsive

### Build (WSL)

```bash
cd /mnt/c/Users/ugosa/Desktop/Cursor\ SOFTWARE/IPPAN
cargo build --release -p ippan-node
cargo build --release -p ippan-txload
```

### Deploy to api1 (WSL, safe + reversible)

```bash
scp target/release/ippan-node root@api1.ippan.uk:/usr/local/bin/ippan-node.new
ssh root@api1.ippan.uk 'set -e; sudo systemctl stop ippan-node; sudo cp -a /usr/local/bin/ippan-node /usr/local/bin/ippan-node.bak.$(date -u +%Y%m%dT%H%M%SZ); sudo install -m 0755 /usr/local/bin/ippan-node.new /usr/local/bin/ippan-node; sudo rm -f /usr/local/bin/ippan-node.new; sudo systemctl start ippan-node; sleep 2; systemctl is-active ippan-node'
```

Verify `/health` + `/status`:

```bash
ssh root@api1.ippan.uk 'curl -sS -m 2 http://127.0.0.1:8080/health; echo; curl -sS -m 2 http://127.0.0.1:8080/status | head -c 400; echo'
```

Deploy txload:

```bash
scp target/release/ippan-txload root@api1.ippan.uk:/usr/local/bin/ippan-txload
ssh root@api1.ippan.uk 'chmod +x /usr/local/bin/ippan-txload'
```

### Server knobs (batch lane)

Enable the devnet-only endpoint:

- `IPPAN_DEV_MODE=true` (or `--dev`)
- `IPPAN_ENABLE_BATCH_SUBMIT=1`

Batch lane overload knobs (defaults shown):

- `IPPAN_BATCH_CONCURRENCY_LIMIT=64`
- `IPPAN_BATCH_QUEUE_CAPACITY=4096`
- `IPPAN_BATCH_BODY_LIMIT_BYTES=33554432` (32 MiB)

Other existing batch knobs (still supported):

- `IPPAN_BATCH_MAX_TX_PER_BATCH=2000`
- `IPPAN_BATCH_BACKPRESSURE_MEMPOOL_SIZE=50000`

### Wire format (must match server)

Binary request body framing:

- `u32 count` (little-endian)
- repeat `count` times:
  - `u32 len` (little-endian)
  - `len` bytes: **bincode (fixint)** serialized `ippan_types::Transaction` (signed)

### Run on api1 (no tunnel)

All scripts:

- pre/post-check `curl $RPC/health`
- stop on `client_errors>0`
- write logs under `/var/lib/ippan/out/batch_<stamp>/`

Example:

```bash
ssh root@api1.ippan.uk 'cd /var/lib/ippan && RPC=http://127.0.0.1:8080 TO_ADDR=<RAW_TO_ADDR> NONCE_START=1 SENDERS_FILE=out/senders/senders.json bash -lc "scripts/ops/ramp_batch_20k.sh"'
ssh root@api1.ippan.uk 'cd /var/lib/ippan && RPC=http://127.0.0.1:8080 TO_ADDR=<RAW_TO_ADDR> NONCE_START=1 SENDERS_FILE=out/senders/senders.json bash -lc "scripts/ops/ramp_batch_50k.sh"'
ssh root@api1.ippan.uk 'cd /var/lib/ippan && RPC=http://127.0.0.1:8080 TO_ADDR=<RAW_TO_ADDR> NONCE_START=1 SENDERS_FILE=out/senders/senders.json bash -lc "scripts/ops/ramp_batch_200k.sh"'
```

### Interpreting `SUMMARY` lines

Each stage prints:

`SUMMARY offered_tps=... accepted_tps=... http_429=... invalid=... client_errors=... dropped_queue_full=...`

- **429 present + client_errors=0**: good overload shedding (no socket collapse)
- **client_errors>0**: ingestion collapse (fix server lane / admission path)
- **dropped_queue_full>0**: client-side backpressure hit (increase `--max-inflight` or lower offered TPS)

### Results table template

| stage | offered_tps | accepted_tps | http_429 | invalid | client_errors | dropped_queue_full | notes |
|------:|------------:|-------------:|---------:|--------:|--------------:|-------------------:|------|
| 20k/20s | | | | | | | |
| 20k/60s | | | | | | | |
| 50k/20s | | | | | | | |
| 50k/60s | | | | | | | |
| 100k/20s | | | | | | | |
| 200k/20s | | | | | | | |
| 200k/60s | | | | | | | |


