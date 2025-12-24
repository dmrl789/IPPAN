#!/usr/bin/env bash
set -euo pipefail

# Multi-sender ramp using ippan-txload --senders-file.
# txload will reserve nonce ranges per sender via POST /nonce/reserve.
#
# Evidence: out/multi_<UTCSTAMP>/<stage>/
# Proofs:
#   - /tx/<hash> for a few hashes per stage
#   - /block/<round> using consensus.round from /status
#   - /account/<HEX> for the first sender (best-effort)

: "${RPC:=http://188.245.97.41:8080}"
: "${SENDERS_FILE:=out/senders/senders.json}"
: "${TO_ADDR:=1GSXnrmfKjH8U1vQVjqL2GGyZ8WD7G9zMVJCSNA4eNVrCLKVtB}"

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="${OUT:-out/multi_${STAMP}}"
mkdir -p "$OUT"
echo "OUT=$OUT"

log() { echo "[$(date -u +%H:%M:%S)] $*"; }

json_pretty() {
  python3 -m json.tool 2>/dev/null || cat
}

first_sender_pubkey_hex() {
  python3 - "$SENDERS_FILE" <<'PY'
import json, sys
senders=json.load(open(sys.argv[1],"r",encoding="utf-8"))
if senders:
    print(senders[0].get("pubkey_hex",""))
PY
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
    if h and h not in hashes:
        hashes.append(h)

try:
    with open(events_path, "r", encoding="utf-8") as f:
        for line in f:
            line=line.strip()
            if not line:
                continue
            try:
                j=json.loads(line)
            except Exception:
                continue
            add(j.get("tx_hash"))
            if len(hashes) >= 10:
                break
except Exception:
    pass

if not hashes:
    try:
        rep=json.load(open(report_path, "r", encoding="utf-8"))
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
p=sys.argv[1]
try:
    j=json.load(open(p, "r", encoding="utf-8"))
except Exception:
    print("")
    raise SystemExit
cons=j.get("consensus") or {}
v=cons.get("round")
if isinstance(v,int):
    print(v); raise SystemExit
for k in ("round","current_round","height","block_height"):
    v=j.get(k)
    if isinstance(v,int):
        print(v); raise SystemExit
print("")
PY
}

run_stage() {
  local name="$1" tps="$2" dur="$3" conc="$4"

  local sdir="$OUT/$name"
  mkdir -p "$sdir"

  log "=== STAGE $name: tps=$tps dur=${dur}s conc=$conc ==="

  curl -sS "$RPC/status" > "$sdir/status_before.json" || true
  cat "$sdir/status_before.json" | json_pretty > "$sdir/status_before.pretty.json" || true

  ./target/release/ippan-txload \
    --rpc "$RPC" \
    --tps "$tps" \
    --duration "$dur" \
    --concurrency "$conc" \
    --senders-file "$SENDERS_FILE" \
    --to "$TO_ADDR" \
    --amount 1 \
    --memo "$name" \
    --nonce-mode provide \
    --report "$sdir/txload_report.json" \
    --events "$sdir/txload_events.jsonl" \
    2>&1 | tee "$sdir/txload_stdout.log"

  curl -sS "$RPC/status" > "$sdir/status_after.json" || true
  cat "$sdir/status_after.json" | json_pretty > "$sdir/status_after.pretty.json" || true

  extract_hashes "$sdir/txload_events.jsonl" "$sdir/txload_report.json" "$sdir/tx_hashes.txt" \
    | tee "$sdir/hash_extract.log" || true

  local i=0
  while read -r h; do
    [[ -z "$h" ]] && continue
    i=$((i+1))
    curl -sS "$RPC/tx/$h" > "$sdir/tx_${i}.json" || true
    cat "$sdir/tx_${i}.json" | json_pretty > "$sdir/tx_${i}.pretty.json" || true
    [[ "$i" -ge 3 ]] && break
  done < "$sdir/tx_hashes.txt"

  local sender0_hex
  sender0_hex="$(first_sender_pubkey_hex)"
  if [[ -n "$sender0_hex" ]]; then
    curl -sS "$RPC/account/$sender0_hex" > "$sdir/account_sender0.json" || true
    cat "$sdir/account_sender0.json" | json_pretty > "$sdir/account_sender0.pretty.json" || true
  fi

  local round
  round="$(get_round_from_status "$sdir/status_after.json")"
  if [[ -n "$round" ]]; then
    curl -sS "$RPC/block/$round" > "$sdir/block_${round}.json" || true
    cat "$sdir/block_${round}.json" | json_pretty > "$sdir/block_${round}.pretty.json" || true
    log "block fetched: $round"
  else
    log "no round in /status; skipped /block/<round>"
  fi

  log "=== STAGE $name COMPLETE: $sdir ==="
  echo
}

log "building ippan-txload..."
cargo build --release -p ippan-txload >/dev/null

# Schedule (fast iteration)
run_stage "tps300_20s"  300  20  800
run_stage "tps500_20s"  500  20  1200
run_stage "tps800_20s"  800  20  1600
run_stage "tps1000_20s" 1000 20  2000
run_stage "tps1500_20s" 1500 20  2000
run_stage "tps2000_20s" 2000 20  2000

log "ALL DONE. Evidence root: $OUT"


