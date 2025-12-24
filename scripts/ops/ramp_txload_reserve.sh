#!/usr/bin/env bash
set -euo pipefail

# Ramp tx acceptance using nonce reservation + provide mode.
# Produces evidence under out/ramp2_<UTCSTAMP>/ and fetches:
# - /tx/<hash> for a few hashes per stage
# - /account/<HEX> for sender history
# - /block/<round> using consensus.round from /status

: "${RPC:=http://188.245.97.41:8080}"

# REQUIRED
: "${PUBKEY_HEX:?set PUBKEY_HEX (sender public key hex for /nonce/reserve)}"
: "${FROM_HEX:?set FROM_HEX (sender account hex for /account/<HEX>)}"

# Keyfile for signing (ippan-wallet JSON keyfile)
: "${WALLET_KEY:=keys/user_wallet.key}"
: "${WALLET_PW_FILE:=keys/user_wallet.pw}"

# Recipient identifier
: "${TO_ADDR:=1GSXnrmfKjH8U1vQVjqL2GGyZ8WD7G9zMVJCSNA4eNVrCLKVtB}"

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="${OUT:-out/ramp2_${STAMP}}"
mkdir -p "$OUT"
echo "OUT=$OUT"

log() { echo "[$(date -u +%H:%M:%S)] $*"; }

if [[ -f "$WALLET_PW_FILE" ]]; then
  export IPPAN_KEY_PASSWORD
  IPPAN_KEY_PASSWORD="$(<"$WALLET_PW_FILE")"
fi

json_pretty() {
  python3 -m json.tool 2>/dev/null || cat
}

reserve_nonce() {
  local count="$1"
  local out_json="$2"

  curl -sS -X POST "$RPC/nonce/reserve" \
    -H 'content-type: application/json' \
    -d "{\"pubkey_hex\":\"$PUBKEY_HEX\",\"count\":$count}" \
    > "$out_json"

  python3 - "$out_json" <<'PY'
import json, sys
j=json.load(open(sys.argv[1], "r", encoding="utf-8"))
print(j["start"])
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
  local name="$1" tps="$2" dur="$3" conc="$4" reserve_count="$5"

  local sdir="$OUT/$name"
  mkdir -p "$sdir"

  log "=== STAGE $name: tps=$tps dur=${dur}s conc=$conc reserve=$reserve_count ==="

  curl -sS "$RPC/status" > "$sdir/status_before.json" || true
  cat "$sdir/status_before.json" | json_pretty > "$sdir/status_before.pretty.json" || true

  local reserve_json="$sdir/nonce_reserve.json"
  local nonce_start
  nonce_start="$(reserve_nonce "$reserve_count" "$reserve_json")"
  echo "$nonce_start" > "$sdir/nonce_start.txt"
  log "nonce_start=$nonce_start"

  ./target/release/ippan-txload \
    --rpc "$RPC" \
    --tps "$tps" \
    --duration "$dur" \
    --concurrency "$conc" \
    --from-key "$WALLET_KEY" \
    --to "$TO_ADDR" \
    --amount 1 \
    --memo "$name" \
    --nonce-mode provide \
    --nonce-start "$nonce_start" \
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

  curl -sS "$RPC/account/$FROM_HEX" > "$sdir/account.json" || true
  cat "$sdir/account.json" | json_pretty > "$sdir/account.pretty.json" || true

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

# Schedule (edit freely)
run_stage "tps100_20s"  100 20  300  5000
run_stage "tps200_20s"  200 20  600  10000
run_stage "tps300_20s"  300 20  800  15000
run_stage "tps400_20s"  400 20  1000 20000

log "ALL DONE. Evidence root: $OUT"


