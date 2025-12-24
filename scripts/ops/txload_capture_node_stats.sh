#!/usr/bin/env bash
set -euo pipefail

# Capture node stats every N seconds (default 2) without jq.
# Usage:
#   ./scripts/ops/txload_capture_node_stats.sh <out_file> [interval_seconds]

OUT_FILE="${1:?out_file required}"
INTERVAL="${2:-2}"

mkdir -p "$(dirname "$OUT_FILE")"

echo "# txload node stats capture" >> "$OUT_FILE"
echo "# started_at_utc=$(date -u +"%Y-%m-%dT%H:%M:%SZ") interval_seconds=$INTERVAL" >> "$OUT_FILE"

while true; do
  ts="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  echo "ts=$ts" >> "$OUT_FILE"
  echo "--- uptime ---" >> "$OUT_FILE"
  uptime >> "$OUT_FILE" 2>&1 || true
  echo "--- vmstat 1 2 (second sample) ---" >> "$OUT_FILE"
  vmstat 1 2 >> "$OUT_FILE" 2>&1 || true
  echo "--- free -m ---" >> "$OUT_FILE"
  free -m >> "$OUT_FILE" 2>&1 || true
  echo "--- df -h ---" >> "$OUT_FILE"
  df -h >> "$OUT_FILE" 2>&1 || true
  echo "--- ss -s ---" >> "$OUT_FILE"
  ss -s >> "$OUT_FILE" 2>&1 || true
  echo "--- ip -s link ---" >> "$OUT_FILE"
  ip -s link >> "$OUT_FILE" 2>&1 || true
  echo "" >> "$OUT_FILE"
  sleep "$INTERVAL"
done


