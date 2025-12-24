#!/usr/bin/env bash
set -euo pipefail

# Ramp real payments using tools/ippan-txload, then prove on-chain objects:
# - GET /tx/<hash>
# - GET /account/<HEX>
# - GET /block/<round>
#
# Evidence is stored under out/ramp_<UTCSTAMP>/.

RPC="${RPC:-http://188.245.97.41:8080}"

# REQUIRED: sender wallet keyfile (ippan-wallet JSON keyfile)
WALLET_KEY="${WALLET_KEY:-keys/user_wallet.key}"
# OPTIONAL: password file for the keyfile (exported via IPPAN_KEY_PASSWORD)
WALLET_PW_FILE="${WALLET_PW_FILE:-keys/user_wallet.pw}"

# REQUIRED: sender account HEX (for /account/<HEX>)
FROM_HEX="${FROM_HEX:-}"

# REQUIRED: recipient identifier (Base58Check, hex, or @handle)
# Default matches the active bot address seen on devnet.
TO_ADDR="${TO_ADDR:-1GSXnrmfKjH8U1vQVjqL2GGyZ8WD7G9zMVJCSNA4eNVrCLKVtB}"

if [[ -z "$FROM_HEX" ]]; then
  echo "ERROR: set FROM_HEX (hex account id) for /account/<HEX>"
  echo "Example: FROM_HEX=c8054c3ad3ac4fafcda284b657547d91b0835fb0d345bf98843eaf2cfe292572"
  exit 1
fi

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="${OUT:-out/ramp_${STAMP}}"
mkdir -p "$OUT"
echo "OUT=$OUT"

log() { echo "[$(date -u +%H:%M:%S)] $*"; }

if [[ -f "$WALLET_PW_FILE" ]]; then
  # txload reads IPPAN_KEY_PASSWORD env
  export IPPAN_KEY_PASSWORD
  IPPAN_KEY_PASSWORD="$(<"$WALLET_PW_FILE")"
fi

json_pretty() {
  python3 -m json.tool 2>/dev/null || cat
}

extract_hashes() {
  local events="$1"
  local report="$2"
  local out_txt="$3"
  python3 - "$events" "$report" "$out_txt" <<'PY'
import json, sys
events_path, report_path, out_path = sys.argv[1:4]

hashes = []

def add(h):
    if not h:
        return
    if h not in hashes:
        hashes.append(h)

# 1) Prefer tx_hash from events JSONL
try:
    with open(events_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                j = json.loads(line)
            except Exception:
                continue
            add(j.get("tx_hash"))
            if len(hashes) >= 10:
                break
except Exception:
    pass

# 2) Fallback: sample_tx_hashes from report
if not hashes:
    try:
        rep = json.load(open(report_path, "r", encoding="utf-8"))
        for h in (rep.get("sample_tx_hashes") or [])[:10]:
            add(h)
    except Exception:
        pass

with open(out_path, "w", encoding="utf-8") as w:
    for h in hashes:
        w.write(h + "\n")

print("found_hashes=", len(hashes))
PY
}

get_round_from_status() {
  local status_json="$1"
  python3 - "$status_json" <<'PY'
import json, sys
p = sys.argv[1]
try:
    j = json.load(open(p, "r", encoding="utf-8"))
except Exception:
    print("")
    raise SystemExit

# Prefer consensus.round
cons = j.get("consensus") or {}
v = cons.get("round")
if isinstance(v, int):
    print(v)
    raise SystemExit

# Fallbacks (older schemas)
for k in ("round", "current_round", "height", "block_height"):
    v = j.get(k)
    if isinstance(v, int):
        print(v)
        raise SystemExit

print("")
PY
}

run_stage() {
  local NAME="$1"
  local TPS="$2"
  local DUR="$3"
  local CONC="$4"
  local MEMO="$5"

  local SDIR="$OUT/${NAME}"
  mkdir -p "$SDIR"

  log "=== STAGE $NAME: tps=$TPS dur=${DUR}s conc=$CONC ==="

  # baseline status
  curl -sS "$RPC/status" > "$SDIR/status_before.json" || true
  cat "$SDIR/status_before.json" | json_pretty > "$SDIR/status_before.pretty.json" || true

  # build txload (quiet)
  cargo build --release -p ippan-txload >/dev/null

  # run txload
  # NOTE: tool expects --rpc (not --rpc-url)
  ./target/release/ippan-txload \
    --rpc "$RPC" \
    --tps "$TPS" \
    --duration "$DUR" \
    --concurrency "$CONC" \
    --from-key "$WALLET_KEY" \
    --to "$TO_ADDR" \
    --amount 1 \
    --memo "$MEMO" \
    --nonce-mode omit \
    --report "$SDIR/txload_report.json" \
    --events "$SDIR/txload_events.jsonl" \
    2>&1 | tee "$SDIR/txload_stdout.log"

  # after status
  curl -sS "$RPC/status" > "$SDIR/status_after.json" || true
  cat "$SDIR/status_after.json" | json_pretty > "$SDIR/status_after.pretty.json" || true

  # Extract hashes
  extract_hashes "$SDIR/txload_events.jsonl" "$SDIR/txload_report.json" "$SDIR/tx_hashes.txt" \
    | tee "$SDIR/hash_extract.log" || true

  # Fetch /tx/<hash> for first 3 hashes
  local i=0
  while read -r H; do
    [[ -z "$H" ]] && continue
    i=$((i+1))
    curl -sS "$RPC/tx/$H" > "$SDIR/tx_${i}.json" || true
    cat "$SDIR/tx_${i}.json" | json_pretty > "$SDIR/tx_${i}.pretty.json" || true
    [[ "$i" -ge 3 ]] && break
  done < "$SDIR/tx_hashes.txt"

  # Pull account (must be hex)
  curl -sS "$RPC/account/$FROM_HEX" > "$SDIR/account.json" || true
  cat "$SDIR/account.json" | json_pretty > "$SDIR/account.pretty.json" || true

  # Fetch block at /status consensus round
  local ROUND
  ROUND="$(get_round_from_status "$SDIR/status_after.json")"
  if [[ -n "$ROUND" ]]; then
    curl -sS "$RPC/block/$ROUND" > "$SDIR/block_${ROUND}.json" || true
    cat "$SDIR/block_${ROUND}.json" | json_pretty > "$SDIR/block_${ROUND}.pretty.json" || true
    log "block fetched: $ROUND -> $SDIR/block_${ROUND}.pretty.json"
  else
    log "no round/height found in /status; skipped /block/<round>"
  fi

  log "=== STAGE $NAME COMPLETE: $SDIR ==="
  echo
}

# RAMP SCHEDULE
run_stage "tps10_10s"  10  10  50  "ramp-10"
run_stage "tps50_20s"  50  20  100 "ramp-50"
run_stage "tps200_30s" 200 30  200 "ramp-200"

log "ALL DONE. Evidence root: $OUT"


