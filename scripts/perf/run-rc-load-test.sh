#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RPC_URL=${RPC_URL:-"http://127.0.0.1:3000"}
TX_COUNT=${TX_COUNT:-10000}
CONCURRENCY=${CONCURRENCY:-32}
AMOUNT=${AMOUNT:-1000}
SIGNING_KEY=${SIGNING_KEY:-}
DESTINATION=${DESTINATION:-}
FEE_LIMIT=${FEE_LIMIT:-}

echo "[perf] building load generator..."
cargo build -p ippan-loadgen --release

LOADGEN_BIN="$ROOT_DIR/target/release/ippan-loadgen"
CMD=("$LOADGEN_BIN" --rpc "$RPC_URL" --tx-count "$TX_COUNT" --concurrency "$CONCURRENCY" --amount "$AMOUNT")

if [[ -n "$SIGNING_KEY" ]]; then
  CMD+=(--signing-key "$SIGNING_KEY")
fi

if [[ -n "$DESTINATION" ]]; then
  CMD+=(--destination "$DESTINATION")
fi

if [[ -n "$FEE_LIMIT" ]]; then
  CMD+=(--fee-limit "$FEE_LIMIT")
fi

echo "[perf] running load test: ${CMD[*]}"
"${CMD[@]}"
