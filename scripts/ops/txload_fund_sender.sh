#!/usr/bin/env bash
set -euo pipefail

# Funds a sender account for txload using the node's /dev/fund endpoint.
#
# IMPORTANT:
# - /dev/fund only accepts loopback requests (127.0.0.1/::1).
# - Run this ON the node (or use SSH port-forward so the node sees loopback).
#
# Required env:
# - IPPAN_RPC_URL (should be http://127.0.0.1:8080 when running via port-forward)
# - IPPAN_SENDER_KEY (path to ippan-wallet keyfile JSON)
#
# Optional env:
# - IPPAN_FUND_AMOUNT (default: 1000000000)
# - IPPAN_FUND_NONCE (optional explicit nonce to set)

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

: "${IPPAN_RPC_URL:?set IPPAN_RPC_URL (must be loopback for /dev/fund)}"
: "${IPPAN_SENDER_KEY:?set IPPAN_SENDER_KEY to sender keyfile path}"

FUND_AMOUNT="${IPPAN_FUND_AMOUNT:-1000000000}"
FUND_NONCE="${IPPAN_FUND_NONCE:-}"

if [[ "$IPPAN_RPC_URL" != http://127.0.0.1* && "$IPPAN_RPC_URL" != http://localhost* ]]; then
  echo "ERROR: /dev/fund only accepts loopback requests."
  echo "Set IPPAN_RPC_URL to http://127.0.0.1:8080 (e.g. via SSH port-forward)."
  echo ""
  echo "Example port-forward:"
  echo "  ssh -L 8080:127.0.0.1:8080 root@<node-ip>"
  exit 2
fi

SENDER_ADDR="$(python3 - <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))["address"])
PY
"$IPPAN_SENDER_KEY")"

REQ="$(python3 - <<'PY'
import json,os,sys
addr=sys.argv[1]
amount=int(os.environ.get("FUND_AMOUNT","1000000000"))
nonce=os.environ.get("FUND_NONCE","").strip()
payload={"address":addr,"amount":amount}
if nonce:
  payload["nonce"]=int(nonce)
print(json.dumps(payload))
PY
"$SENDER_ADDR")"

echo "Funding sender via /dev/fund"
echo "  rpc:    $IPPAN_RPC_URL"
echo "  addr:   $SENDER_ADDR"
echo "  amount: $FUND_AMOUNT"

curl -sS -X POST "$IPPAN_RPC_URL/dev/fund" \
  -H "Content-Type: application/json" \
  -d "$REQ"
echo ""


