#!/usr/bin/env bash
set -euo pipefail

# Fund all senders from out/senders/senders.json via /dev/fund.
#
# /dev/fund is loopback-only, so set RPC to your SSH-forwarded endpoint, e.g.:
#   export RPC="http://127.0.0.1:18080"
#
# Artifacts:
#   out/senders/fund_results.jsonl

: "${RPC:=http://127.0.0.1:18080}"
: "${SENDERS_FILE:=out/senders/senders.json}"
: "${AMOUNT:=100000}"
: "${PARALLEL:=8}"

host="$(python3 - <<PY
import sys, urllib.parse
u=urllib.parse.urlparse(sys.argv[1])
print(u.hostname or "")
PY
"$RPC")"

if [[ "$host" != "127.0.0.1" && "$host" != "localhost" && "${FORCE_NON_LOOPBACK:-0}" != "1" ]]; then
  echo "Refusing to call /dev/fund on non-loopback RPC ($RPC)."
  echo "Use SSH port-forward so RPC is localhost, or set FORCE_NON_LOOPBACK=1 if you know what you're doing."
  exit 1
fi

if [[ ! -f "$SENDERS_FILE" ]]; then
  echo "Missing SENDERS_FILE: $SENDERS_FILE"
  exit 1
fi

OUT_DIR="$(dirname "$SENDERS_FILE")"
OUT_JSONL="$OUT_DIR/fund_results.jsonl"
tmp_addrs="$(mktemp)"

python3 - "$SENDERS_FILE" > "$tmp_addrs" <<'PY'
import json, sys
senders=json.load(open(sys.argv[1],"r",encoding="utf-8"))
for s in senders:
    addr=s.get("address") or ""
    if addr:
        print(addr)
PY

echo "RPC=$RPC"
echo "SENDERS_FILE=$SENDERS_FILE"
echo "AMOUNT=$AMOUNT"
echo "PARALLEL=$PARALLEL"
echo "results -> $OUT_JSONL"

rm -f "$OUT_JSONL"

fund_one() {
  local addr="$1"
  local body
  body="$(printf '{"address":"%s","amount":%s}' "$addr" "$AMOUNT")"
  curl -sS -X POST "$RPC/dev/fund" -H 'content-type: application/json' -d "$body" \
    | python3 -c 'import json,sys; j=json.load(sys.stdin); print(json.dumps(j, separators=(",",":")))' \
    || printf '{"error":"fund_failed","address":"%s"}\n' "$addr"
}

export -f fund_one
export RPC AMOUNT

# shellcheck disable=SC2016
cat "$tmp_addrs" | xargs -I{} -P "$PARALLEL" bash -lc 'fund_one "$@"' _ {} >> "$OUT_JSONL"

rm -f "$tmp_addrs"

echo "Done. Wrote $OUT_JSONL"
tail -n 5 "$OUT_JSONL" || true


