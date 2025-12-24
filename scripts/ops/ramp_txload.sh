#!/usr/bin/env bash
set -euo pipefail

# Ramp txload safely up to 2000 TPS using the standard runner.
#
# Required env (same as run_txload_2000tps.sh):
# - IPPAN_RPC_URL
# - IPPAN_SENDER_KEY
# - IPPAN_RECEIVER_ADDR
# Optional:
# - IPPAN_KEY_PASSWORD

: "${IPPAN_RPC_URL:?set IPPAN_RPC_URL}"
: "${IPPAN_SENDER_KEY:?set IPPAN_SENDER_KEY}"
: "${IPPAN_RECEIVER_ADDR:?set IPPAN_RECEIVER_ADDR}"

SCHEDULE=(
  "20 5 50"
  "200 30 200"
  "500 60 300"
  "1000 120 400"
  "2000 600 800"
)

for row in "${SCHEDULE[@]}"; do
  read -r TPS DURATION CONCURRENCY <<<"$row"
  echo ""
  echo "=== RAMP: TPS=$TPS DURATION=$DURATION CONCURRENCY=$CONCURRENCY ==="
  export IPPAN_TXLOAD_TPS="$TPS"
  export IPPAN_TXLOAD_DURATION="$DURATION"
  export IPPAN_TXLOAD_CONCURRENCY="$CONCURRENCY"
  ./scripts/ops/run_txload_2000tps.sh
done


