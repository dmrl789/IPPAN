## CURSOR IDE ORDER — DevNet TPS Ramp (20 → 50 → 100 → 200)

### GLOBAL RULES

- Never ramp TPS unless **invariants are green**.
- Prefer an SSH tunnel RPC (`127.0.0.1:18080`) over public `:8080` until you prove public stability.
- For every stage, capture: `accepted_tps`, `http_429`, and any client errors.
- If `/health` fails twice or `round` stops advancing → **STOP** (do not escalate).

---

### STEP 0 — Pull latest + build tools (WSL)

```bash
git fetch --all
git status --porcelain
# Expect clean. If not clean: STOP and report what's modified.

cargo build --release -p ippan-txload
```

---

### STEP 1 — Verify invariants (mandatory gate)

```bash
./scripts/ops/devnet_invariant_check.sh api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk
```

Pass condition: prints `✅ DEVNET INVARIANTS OK`

---

### STEP 2 — Use a stable RPC path (tunnel)

Terminal A:

```bash
ssh -N -L 18080:127.0.0.1:8080 root@api1.ippan.uk
```

Terminal B:

```bash
curl -sS --connect-timeout 2 --max-time 4 http://127.0.0.1:18080/health; echo
curl -sS --connect-timeout 2 --max-time 4 http://127.0.0.1:18080/status | head -c 250; echo
```

---

### STEP 3 — Ramp schedule (single-sender)

Create log folder:

```bash
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
ROOT="out/ramp_small_${STAMP}"
mkdir -p "$ROOT"
```

Each stage:

```bash
RPC="http://127.0.0.1:18080" ./scripts/ops/ramp_txload_reserve.sh <TPS> <SECONDS> | tee "$ROOT/<name>.log"
```

Stages:

```bash
RPC="http://127.0.0.1:18080" ./scripts/ops/ramp_txload_reserve.sh 20 30  | tee "$ROOT/tps20_30s.log"
RPC="http://127.0.0.1:18080" ./scripts/ops/ramp_txload_reserve.sh 50 30  | tee "$ROOT/tps50_30s.log"
RPC="http://127.0.0.1:18080" ./scripts/ops/ramp_txload_reserve.sh 100 30 | tee "$ROOT/tps100_30s.log"
RPC="http://127.0.0.1:18080" ./scripts/ops/ramp_txload_reserve.sh 200 30 | tee "$ROOT/tps200_30s.log"
```

Stop conditions:
- accepted TPS collapses unexpectedly
- connection resets / timeouts appear
- `/health` fails twice
- `round` stops advancing

---

### STEP 4 — Quick health-under-load regression

```bash
RPC="http://127.0.0.1:18080" ./scripts/ops/health_under_load.sh | tee "$ROOT/health_under_load.log"
```

---

### STEP 5 — If single-sender plateaus: multi-sender (optional)

Generate senders:

```bash
./scripts/ops/multi_generate_senders.sh 20
```

Fund senders via `/dev/fund` (requires dev mode and tunnel):

```bash
export RPC="http://127.0.0.1:18080"
export AMOUNT="200000"
./scripts/ops/multi_fund_senders.sh
```

Run multi-sender (simple parallel split):

```bash
RPC="http://127.0.0.1:18080" SENDERS_FILE="out/senders/senders.json" TPS_TOTAL=200 SECONDS=30 \
./scripts/ops/ramp_txload_multisender.sh | tee "$ROOT/multisender_ramp.log"
```

---

### STEP 6 — What to paste back

1) Invariants output:

```bash
./scripts/ops/devnet_invariant_check.sh api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk
```

2) The `SUMMARY ...` line from each stage log.


