#!/usr/bin/env bash
set -euo pipefail

# Rate-limited tx ramp stage using ippan-txload.
#
# Usage:
#   RPC="http://127.0.0.1:18080" SIGNING_KEY_FILE="out/devnet_sender.key" ./scripts/ops/ramp_txload_reserve.sh 50 30
#
# If SIGNING_KEY_FILE is not provided, we'll try to create out/devnet_sender.key by
# fetching the devnet validator key from api1 via ssh (root required).

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <tps> <seconds>" >&2
  exit 2
fi

TPS="$1"
SECONDS="$2"

RPC="${RPC:-}"
if [[ -z "$RPC" ]]; then
  echo "ERROR: RPC env var required (e.g. http://127.0.0.1:18080)" >&2
  exit 2
fi

SIGNING_KEY_FILE="${SIGNING_KEY_FILE:-out/devnet_sender.key}"
if [[ ! -f "$SIGNING_KEY_FILE" ]]; then
  mkdir -p "$(dirname "$SIGNING_KEY_FILE")"
  echo "SIGNING_KEY_FILE not found; attempting to fetch from api1.ippan.uk (/var/lib/ippan/data/devnet/validator.key)..." >&2
  ssh root@api1.ippan.uk 'set -euo pipefail; sudo cat /var/lib/ippan/data/devnet/validator.key' >"$SIGNING_KEY_FILE"
  chmod 600 "$SIGNING_KEY_FILE" || true
fi

CONCURRENCY="${CONCURRENCY:-16}"
AMOUNT="${AMOUNT:-1000}"
FEE="${FEE:-}"

cmd=(cargo run --release -p ippan-txload -- run
  --rpc "$RPC"
  --tps "$TPS"
  --seconds "$SECONDS"
  --concurrency "$CONCURRENCY"
  --amount "$AMOUNT"
  --signing-key-file "$SIGNING_KEY_FILE"
)

if [[ -n "$FEE" ]]; then
  cmd+=(--fee "$FEE")
fi

echo "Running: ${cmd[*]}" >&2
exec "${cmd[@]}"


