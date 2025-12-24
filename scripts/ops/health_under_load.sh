#!/usr/bin/env bash
set -euo pipefail

# Regression guard: ensure /health stays responsive under sustained tx load.
#
# Usage:
#   SENDERS_FILE=out/senders.json TO=@sink.ipn ./scripts/ops/health_under_load.sh http://127.0.0.1:8080
#
# Optional env:
#   TPS=500 DURATION=20 CONCURRENCY=200 HEALTH_HZ=10 NONCE_MODE=omit
#
# Notes:
# - Uses `cargo run -p ippan-txload --release` by default.
# - Requires a multi-sender JSON file so we don't depend on interactive key passwords.

RPC_BASE="${RPC:-${1:-}}"
if [ -z "$RPC_BASE" ]; then
  RPC_BASE="http://127.0.0.1:18080"
fi
RPC_BASE="${RPC_BASE%/}"

SENDERS_FILE="${SENDERS_FILE:-out/senders/senders.json}"
TO_ADDR="${TO_ADDR:-${TO:-}}"
if [ -z "$TO_ADDR" ]; then
  # Match ramp default.
  TO_ADDR="1GSXnrmfKjH8U1vQVjqL2GGyZ8WD7G9zMVJCSNA4eNVrCLKVtB"
fi

TPS="${TPS:-500}"
DURATION="${DURATION:-20}"
CONCURRENCY="${CONCURRENCY:-200}"
HEALTH_HZ="${HEALTH_HZ:-10}"
NONCE_MODE="${NONCE_MODE:-provide}"

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required" >&2
  exit 2
fi

tmp_times="$(mktemp)"
trap 'rm -f "$tmp_times"' EXIT

report_dir="out/ops"
mkdir -p "$report_dir"
tx_report="$report_dir/txload_report.json"
tx_events="$report_dir/txload_events.jsonl"

echo "Starting tx load: tps=$TPS duration=${DURATION}s concurrency=$CONCURRENCY rpc=$RPC_BASE"
(
  set -euo pipefail
  if [ -x ./target/release/ippan-txload ]; then
    txload_bin=./target/release/ippan-txload
  else
    txload_bin=./target/debug/ippan-txload
  fi

  "$txload_bin" \
      --rpc "$RPC_BASE" \
      --tps "$TPS" \
      --duration "$DURATION" \
      --concurrency "$CONCURRENCY" \
      --senders-file "$SENDERS_FILE" \
      --to "$TO_ADDR" \
      --amount 1 \
      --memo "health_under_load" \
      --nonce-mode "$NONCE_MODE" \
      --report "$tx_report" \
      --events "$tx_events" \
      >/dev/null
) &
tx_pid="$!"

failures=0
interval_s="$(awk -v hz="$HEALTH_HZ" 'BEGIN { if (hz<=0) hz=10; printf "%.3f", 1.0/hz }')"

echo "Probing /health at ${HEALTH_HZ}Hz for ${DURATION}s (interval=${interval_s}s)"
start_epoch="$(date +%s)"
end_epoch="$((start_epoch + DURATION))"
while [ "$(date +%s)" -lt "$end_epoch" ]; do
  t="$(curl -sS -o /dev/null -w "%{time_total}" --max-time 1 "$RPC_BASE/health" || echo "FAIL")"
  if [ "$t" = "FAIL" ]; then
    failures="$((failures + 1))"
  else
    echo "$t" >>"$tmp_times"
  fi
  sleep "$interval_s"
done

wait "$tx_pid" || true

n="$(wc -l <"$tmp_times" | tr -d ' ')"
if [ "$n" -eq 0 ]; then
  echo "No successful /health samples collected (failures=$failures)" >&2
  exit 1
fi

p99_idx="$(( (n * 99 + 99) / 100 ))" # ceil(0.99*n)
p99_s="$(sort -n "$tmp_times" | sed -n "${p99_idx}p")"
p99_ms="$(awk -v t="$p99_s" 'BEGIN { printf "%.0f", t * 1000.0 }')"

echo "Results: samples=$n failures=$failures p99=${p99_ms}ms"

if [ "$failures" -ne 0 ]; then
  echo "FAIL: /health had ${failures} timeouts/errors under load" >&2
  exit 1
fi
if [ "$p99_ms" -ge 200 ]; then
  echo "FAIL: /health p99 ${p99_ms}ms >= 200ms under load" >&2
  exit 1
fi

echo "PASS: /health remained responsive under load"


