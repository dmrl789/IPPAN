#!/usr/bin/env bash
set -euo pipefail

# Multi-sender ramp runner: splits load across generated/funded senders.
#
# NOTE: Current ippan-txload tool supports only single-sender `run` mode.
# This script is a placeholder that will run multiple single-sender instances
# (one per sender) with reduced TPS per sender.
#
# Usage:
#   RPC="http://127.0.0.1:18080" SENDERS_FILE="out/senders/senders.json" TPS_TOTAL=200 SECONDS=30 ./scripts/ops/ramp_txload_multisender.sh

RPC="${RPC:-}"
if [[ -z "$RPC" ]]; then
  echo "ERROR: RPC env var required" >&2
  exit 2
fi

SENDERS_FILE="${SENDERS_FILE:-out/senders/senders.json}"
if [[ ! -f "$SENDERS_FILE" ]]; then
  echo "ERROR: missing senders file: $SENDERS_FILE" >&2
  exit 2
fi

TPS_TOTAL="${TPS_TOTAL:-200}"
SECONDS="${SECONDS:-30}"
CONCURRENCY="${CONCURRENCY:-8}"
AMOUNT="${AMOUNT:-1000}"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

# Extract signing keys into per-sender files (so the rust tool can read them easily).
python3 - "$SENDERS_FILE" "$tmp_dir" <<'PY'
import json,sys,os
senders_file=sys.argv[1]
out_dir=sys.argv[2]
with open(senders_file,"r",encoding="utf-8") as f:
    senders=json.load(f)
for i,s in enumerate(senders):
    sk=s.get("signing_key_hex")
    if not sk: 
        continue
    p=os.path.join(out_dir, f"sender_{i}.key")
    with open(p,"w",encoding="utf-8") as wf:
        wf.write(sk.strip()+"\n")
PY

sender_files=( "$tmp_dir"/sender_*.key )
count="${#sender_files[@]}"
if [[ "$count" -le 0 ]]; then
  echo "ERROR: no signing keys found in senders file" >&2
  exit 2
fi

# Split TPS roughly evenly across senders (floor), ensure at least 1.
TPS_PER=$(( TPS_TOTAL / count ))
if [[ "$TPS_PER" -lt 1 ]]; then TPS_PER=1; fi

echo "senders: $count"
echo "tps_total: $TPS_TOTAL"
echo "tps_per_sender: $TPS_PER"

pids=()
for f in "${sender_files[@]}"; do
  cargo run --release -p ippan-txload -- run \
    --rpc "$RPC" \
    --tps "$TPS_PER" \
    --seconds "$SECONDS" \
    --concurrency "$CONCURRENCY" \
    --amount "$AMOUNT" \
    --signing-key-file "$f" \
    >"$f.log" 2>&1 &
  pids+=( "$!" )
done

rc=0
for p in "${pids[@]}"; do
  wait "$p" || rc=1
done

grep -h "SUMMARY " "$tmp_dir"/*.log || true
exit "$rc"


