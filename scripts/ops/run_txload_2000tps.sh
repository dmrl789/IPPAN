#!/usr/bin/env bash
set -euo pipefail

# One-command runner for a 2000 TPS / 10 minute txload on IPPAN DevNet.
#
# Required env:
# - IPPAN_RPC_URL
# - IPPAN_SENDER_KEY   (path to ippan-wallet keyfile JSON)
# - IPPAN_RECEIVER_ADDR (Base58Check, hex, or @handle)
#
# Optional env:
# - IPPAN_TXLOAD_TPS (default 2000)
# - IPPAN_TXLOAD_DURATION (default 600)
# - IPPAN_TXLOAD_CONCURRENCY (default 200)
# - IPPAN_TXLOAD_AMOUNT (default 1)
# - IPPAN_TXLOAD_MEMO (default loadtest)
# - IPPAN_KEY_PASSWORD (optional; unlock encrypted keyfile)

: "${IPPAN_RPC_URL:?set IPPAN_RPC_URL}"
: "${IPPAN_SENDER_KEY:?set IPPAN_SENDER_KEY to sender keyfile path}"
: "${IPPAN_RECEIVER_ADDR:?set IPPAN_RECEIVER_ADDR to receiver address/handle}"

TPS="${IPPAN_TXLOAD_TPS:-2000}"
DURATION="${IPPAN_TXLOAD_DURATION:-600}"
CONCURRENCY="${IPPAN_TXLOAD_CONCURRENCY:-200}"
AMOUNT="${IPPAN_TXLOAD_AMOUNT:-1}"
MEMO="${IPPAN_TXLOAD_MEMO:-loadtest}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

TS="$(date -u +"%Y%m%dT%H%M%SZ")"
OUT_DIR="out/txload_${TS}"
mkdir -p "$OUT_DIR"

NODE_STATS_LOG="$OUT_DIR/node_stats.log"
RPC_HEALTH_LOG="$OUT_DIR/rpc_health.log"
REPORT_JSON="$OUT_DIR/txload_report.json"
EVENTS_JSONL="$OUT_DIR/txload_events.jsonl"
STDOUT_LOG="$OUT_DIR/txload_stdout.log"

echo "Output dir: $OUT_DIR"

./scripts/ops/txload_capture_node_stats.sh "$NODE_STATS_LOG" 2 &
PID_NODE_STATS=$!

./scripts/ops/txload_capture_rpc_health.sh "$IPPAN_RPC_URL" "$RPC_HEALTH_LOG" 1 &
PID_RPC_HEALTH=$!

cleanup() {
  kill "$PID_NODE_STATS" "$PID_RPC_HEALTH" >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "Running ippan-txload..."
(
  cargo run -q -p ippan-txload --release -- \
  --rpc "$IPPAN_RPC_URL" \
  --tps "$TPS" \
  --duration "$DURATION" \
  --concurrency "$CONCURRENCY" \
  --from-key "$IPPAN_SENDER_KEY" \
  --to "$IPPAN_RECEIVER_ADDR" \
  --amount "$AMOUNT" \
  --memo "$MEMO" \
  --nonce-mode omit \
  --report "$REPORT_JSON" \
  --events "$EVENTS_JSONL"
 ) 2>&1 | tee "$STDOUT_LOG"

echo ""
echo "Artifacts:"
echo "  report:      $REPORT_JSON"
echo "  events:      $EVENTS_JSONL"
echo "  stdout:      $STDOUT_LOG"
echo "  node stats:  $NODE_STATS_LOG"
echo "  rpc health:  $RPC_HEALTH_LOG"

echo ""
echo "== evidence tail =="
echo "-- report (last 60 lines) --"
tail -n 60 "$REPORT_JSON" 2>/dev/null || true
echo ""
echo "-- events (first 5 lines) --"
head -n 5 "$EVENTS_JSONL" 2>/dev/null || true
echo ""
echo "-- events (last 5 lines) --"
tail -n 5 "$EVENTS_JSONL" 2>/dev/null || true
echo ""
echo "-- stdout (last 40 lines) --"
tail -n 40 "$STDOUT_LOG" 2>/dev/null || true


