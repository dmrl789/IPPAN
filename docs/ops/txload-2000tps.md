## IPPAN DevNet true transaction load test @ 2000 TPS (10 minutes)

This runbook produces a **real transaction stream** against an IPPAN node’s **`POST /tx/payment`** endpoint, targeting **2000 TPS** for **600 seconds**, while capturing:

- **client-side latency** (p50/p95/p99)
- **accept/reject rate** + error breakdown
- **mempool depth + consensus round** via `GET /status`
- **node resource stats** (CPU/RAM/load/disk/net)

---

## Prerequisites

- **WSL Ubuntu** (or a Linux host)
- Tools:
  - `curl`
  - `python3`
  - `vmstat` (package `procps`)
  - `ip` + `ss` (package `iproute2`)
- Rust toolchain working for this repo (`cargo`)

On Ubuntu/WSL:

```bash
sudo apt-get update
sudo apt-get install -y curl python3 procps iproute2
```

---

## RPC endpoints used

- **Submit payments**: `POST /tx/payment`
- **Health + mempool**: `GET /status` (includes `mempool_size`, `consensus.round`, etc.)
- **Health snapshot**: `GET /health`
- **Fast nonce**: `GET /nonce/<pubkey_hex>` (lightweight nonce lookup; no tx history)
- **Nonce reservation**: `POST /nonce/reserve` (atomic range reservation; best for high TPS)

Note: the dev-only funding endpoint exists but is loopback-only:

- **Dev funding (loopback only)**: `POST /dev/fund` (requires `IPPAN_DEV_MODE=true` and requests from `127.0.0.1`)

---

## Step 1 — Generate sender + receiver keyfiles (do not commit)

This generates **ippan-wallet JSON keyfiles** into a gitignored folder.

```bash
./scripts/ops/txload_generate_keys.sh
```

It prints environment variables you can export:

- `IPPAN_SENDER_KEY` (path to sender keyfile)
- `IPPAN_RECEIVER_ADDR` (receiver address string)

---

## Step 2 — Fund the sender

### Option A: Faucet flow (if you have one)

If you already have a faucet, fund the **sender address** printed in Step 1.

### Option B: Use `/dev/fund` (recommended for devnet nodes you control)

`/dev/fund` only accepts loopback requests. Do one of:

- Run the script **on the node** where RPC is listening.
- Or SSH port-forward so your laptop hits **127.0.0.1** from the node’s perspective:

```bash
ssh -L 8080:127.0.0.1:8080 root@<node-ip>
export IPPAN_RPC_URL="http://127.0.0.1:8080"
./scripts/ops/txload_fund_sender.sh
```

You can tune the funded balance (note: `/dev/fund` sets the balance):

```bash
export IPPAN_FUND_AMOUNT="1000000000"
./scripts/ops/txload_fund_sender.sh
```

---

## Step 3 — Run the 2000 TPS test (one command)

```bash
export IPPAN_RPC_URL="http://188.245.97.41:8080"
export IPPAN_SENDER_KEY="out/txload/keys/sender.key"
export IPPAN_RECEIVER_ADDR="@loadtestreceiver.ipn"   # or a Base58Check/hex address

./scripts/ops/run_txload_2000tps.sh
```

Artifacts are written under `out/txload_<timestamp>/`:

- `txload_report.json` (summary)
- `txload_events.jsonl` (per-request events)
- `txload_stdout.log` (full stdout/stderr captured via `tee`)
- `node_stats.log`
- `rpc_health.log`

### Nonce handling

The runner defaults to `ippan-txload --nonce-mode omit`, which **omits the `nonce` field** from the `POST /tx/payment` request so the node derives the nonce. This avoids any dependency on `GET /account/...` nonce discovery.

For **serious throughput tests**, prefer nonce reservation:

1) Reserve a nonce range:

```bash
curl -sS -X POST "$IPPAN_RPC_URL/nonce/reserve" \
  -H "Content-Type: application/json" \
  -d "{\"pubkey_hex\":\"<SENDER_PUBKEY_HEX>\",\"count\":50000}"
```

2) Run `ippan-txload` with `--nonce-mode provide --nonce-start <START>` (or set it via your wrapper).

---

## Tuning RPC admission (accepted TPS)

The `/tx/payment` path uses a **bounded admission queue**. Under load, the node should return **HTTP 429** (bounded backpressure) instead of collapsing with 503s.

### Env knobs (node process)

- `IPPAN_PAYMENT_ADMISSION_CAPACITY` (default: **10_000**; clamp: 1_000..1_000_000)
- `IPPAN_PAYMENT_ADMISSION_WORKERS` (default: **8**; clamp: 1..256)

### Verify effective settings

```bash
curl -s http://127.0.0.1:8080/status | head -c 1200; echo
```

Look for:

- `rpc_queue_depth`
- `rpc_queue_capacity`
- `rpc_queue_workers`

### Suggested starting points

- 10–200 TPS: capacity **10_000**, workers **8**
- 200–500 TPS: capacity **50_000**, workers **16**
- 500+ TPS: capacity **100_000**, workers **32** (watch CPU + latency)

## What “success” looks like

- **Accepted rate**: >= 95% accepted-to-mempool (or a clearly explained limit, e.g. rate-limiter / overload)
- **Latency**: p50/p95/p99 in `txload_report.json`
- **Chain progress evidence**:
  - `rpc_health.log` shows `/status` values changing (e.g., `consensus.round`)
  - mempool depth behavior is visible via `/status.mempool_size`

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| HTTP 429 | rate limiting | Reduce `IPPAN_TXLOAD_CONCURRENCY`, confirm RPC rate limits, or scale gateway |
| HTTP 503 | circuit breaker / overload | Reduce TPS/concurrency, check node CPU/RAM, inspect logs |
| Error `account not found` | sender not funded | Fund sender, confirm you’re hitting the correct RPC node |
| `fee too low` errors | fee policy rejects limit | Omit `--fee` or set it above required minimum |
| `mempool rejected transaction` | mempool saturated/duplicate/nonce rules | Ensure nonce starts at current nonce+1, reduce TPS |

### Interpreting overload correctly

For benchmarking, **429 is good** (bounded backpressure). It means the node is protecting itself while staying responsive.

**503 should be rare** and indicates a real server-side failure path (timeouts/crashes).

---

## Notes on determinism

`ippan-txload` uses a **1ms pacer** to emit tokens at the target TPS with **remainder carry** to avoid drift, and uses a **monotonic nonce** sequence to ensure unique transactions.


