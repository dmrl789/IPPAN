#!/usr/bin/env bash
set -euo pipefail

# Capture RPC health every N seconds (default 1) without jq.
# Usage:
#   ./scripts/ops/txload_capture_rpc_health.sh <rpc_base_url> <out_file> [interval_seconds]

RPC_URL="${1:?rpc_base_url required}"
OUT_FILE="${2:?out_file required}"
INTERVAL="${3:-1}"

mkdir -p "$(dirname "$OUT_FILE")"

echo "# txload rpc health capture" >> "$OUT_FILE"
echo "# started_at_utc=$(date -u +"%Y-%m-%dT%H:%M:%SZ") interval_seconds=$INTERVAL rpc=$RPC_URL" >> "$OUT_FILE"

while true; do
  ts="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  echo "ts=$ts" >> "$OUT_FILE"

  echo "--- GET /status ---" >> "$OUT_FILE"
  curl -sS --max-time 2 "$RPC_URL/status" >> "$OUT_FILE" 2>&1 || true
  echo "" >> "$OUT_FILE"

  echo "--- GET /health ---" >> "$OUT_FILE"
  curl -sS --max-time 2 "$RPC_URL/health" >> "$OUT_FILE" 2>&1 || true
  echo "" >> "$OUT_FILE"

  sleep "$INTERVAL"
done


