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
#
# NOTE: Some deployments may have slow/hanging /nonce endpoints. This script assumes the
# generated+funded senders are fresh (nonce=0), so we use --senders-nonce-start with deterministic
# per-stage offsets (1 + used_offset).
#

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="${OUT:-out/multi_${STAMP}}"
mkdir -p "$OUT"
echo "OUT=$OUT"

log() { echo "[$(date -u +%H:%M:%S)] $*"; }

json_pretty() {
  python3 -m json.tool 2>/dev/null || cat
}

curl_status() {
  # Keep /status from hanging the whole ramp if RPC is temporarily overloaded.
  curl -sS --connect-timeout 3 --max-time 5 "$@"
}

curl_get() {
  # Generic GET with sane timeouts for proof fetches.
  curl -sS --connect-timeout 3 --max-time 10 "$@"
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

make_stage_senders_file() {
  local base_file="$1"
  local starts_file="$2"
  local used_offset="$3"
  local out_file="$4"

  python3 - "$base_file" "$starts_file" "$used_offset" "$out_file" <<'PY'
import json, sys
base_path, starts_path, used_offset_s, out_path = sys.argv[1:5]
used_offset=int(used_offset_s)
base=json.load(open(base_path,"r",encoding="utf-8"))
starts={}
for line in open(starts_path,"r",encoding="utf-8"):
    parts=line.strip().split()
    if len(parts)!=2: continue
    starts[parts[0]]=int(parts[1])
for s in base:
    pub=s["pubkey_hex"]
    if pub not in starts:
        raise SystemExit(f"missing nonce start for {pub}")
    s["nonce_start"]=starts[pub]+used_offset
json.dump(base, open(out_path,"w",encoding="utf-8"), indent=2)
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
  local name="$1" tps="$2" dur="$3" conc="$4" stage_senders_file="$5"

  local sdir="$OUT/$name"
  mkdir -p "$sdir"

  log "=== STAGE $name: tps=$tps dur=${dur}s conc=$conc ==="

  curl_status "$RPC/status" > "$sdir/status_before.json" || true
  cat "$sdir/status_before.json" | json_pretty > "$sdir/status_before.pretty.json" || true

  ./target/release/ippan-txload \
    --rpc "$RPC" \
    --tps "$tps" \
    --duration "$dur" \
    --concurrency "$conc" \
    --senders-file "$stage_senders_file" \
    --senders-nonce-start "$((1 + USED_OFFSET))" \
    --to "$TO_ADDR" \
    --amount 1 \
    --memo "$name" \
    --nonce-mode provide \
    --report "$sdir/txload_report.json" \
    --events "$sdir/txload_events.jsonl" \
    2>&1 | tee "$sdir/txload_stdout.log"

  curl_status "$RPC/status" > "$sdir/status_after.json" || true
  cat "$sdir/status_after.json" | json_pretty > "$sdir/status_after.pretty.json" || true

  extract_hashes "$sdir/txload_events.jsonl" "$sdir/txload_report.json" "$sdir/tx_hashes.txt" \
    | tee "$sdir/hash_extract.log" || true

  local i=0
  while read -r h; do
    [[ -z "$h" ]] && continue
    i=$((i+1))
    curl_get "$RPC/tx/$h" > "$sdir/tx_${i}.json" || true
    cat "$sdir/tx_${i}.json" | json_pretty > "$sdir/tx_${i}.pretty.json" || true
    [[ "$i" -ge 3 ]] && break
  done < "$sdir/tx_hashes.txt"

  local sender0_hex
  sender0_hex="$(first_sender_pubkey_hex)"
  if [[ -n "$sender0_hex" ]]; then
    curl_get "$RPC/account/$sender0_hex" > "$sdir/account_sender0.json" || true
    cat "$sdir/account_sender0.json" | json_pretty > "$sdir/account_sender0.pretty.json" || true
  fi

  local round
  round="$(get_round_from_status "$sdir/status_after.json")"
  if [[ -n "$round" ]]; then
    curl_get "$RPC/block/$round" > "$sdir/block_${round}.json" || true
    cat "$sdir/block_${round}.json" | json_pretty > "$sdir/block_${round}.pretty.json" || true
    log "block fetched: $round"
  else
    log "no round in /status; skipped /block/<round>"
  fi

  log "=== STAGE $name COMPLETE: $sdir ==="
  echo
}

log "building ippan-txload..."
if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: cargo not found in PATH"
  exit 1
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo "ERROR: python3 not found in PATH"
  exit 1
fi

cargo build --release -p ippan-txload

if [[ ! -x ./target/release/ippan-txload ]]; then
  echo "ERROR: missing ./target/release/ippan-txload after build"
  ls -la ./target/release || true
  exit 1
fi

# Fetch starting nonces ONCE for all senders, then create per-stage senders files with adjusted
# nonce_start offsets so we never call /nonce/reserve (which can be slow on some nodes).
SENDERS_N="$(python3 -c 'import json; j=json.load(open("out/senders/senders.json")); print(len(j))' 2>/dev/null || echo 0)"
if [[ "$SENDERS_N" -le 0 ]]; then
  echo "ERROR: could not read senders count from $SENDERS_FILE"
  exit 1
fi

USED_OFFSET=0

# Schedule (fast iteration)
run_stage "tps300_20s"  300  20  200 "$SENDERS_FILE"
USED_OFFSET=$(( USED_OFFSET + (300*20 + SENDERS_N - 1) / SENDERS_N ))

run_stage "tps500_20s"  500  20  300 "$SENDERS_FILE"
USED_OFFSET=$(( USED_OFFSET + (500*20 + SENDERS_N - 1) / SENDERS_N ))

run_stage "tps800_20s"  800  20  400 "$SENDERS_FILE"
USED_OFFSET=$(( USED_OFFSET + (800*20 + SENDERS_N - 1) / SENDERS_N ))

run_stage "tps1000_20s" 1000 20  500 "$SENDERS_FILE"
USED_OFFSET=$(( USED_OFFSET + (1000*20 + SENDERS_N - 1) / SENDERS_N ))

run_stage "tps1500_20s" 1500 20  600 "$SENDERS_FILE"
USED_OFFSET=$(( USED_OFFSET + (1500*20 + SENDERS_N - 1) / SENDERS_N ))

run_stage "tps2000_20s" 2000 20  600 "$SENDERS_FILE"

log "ALL DONE. Evidence root: $OUT"


