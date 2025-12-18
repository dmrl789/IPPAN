#!/usr/bin/env bash
set -euo pipefail

# One-shot: collect /status and /ai/status from all devnet nodes via SSH.
# Writes JSON snapshots under logs/ai_shadow/.

NODES=(
  "root@5.223.51.238"
  "root@188.245.97.41"
  "root@135.181.145.174"
  "root@178.156.219.107"
)

LOG_DIR="logs/ai_shadow"
mkdir -p "${LOG_DIR}"

TS_UTC="$(date -u +"%Y-%m-%dT%H-%M-%SZ")"

for NODE in "${NODES[@]}"; do
  SAFE_NODE="${NODE//@/_}" # replace @ with _ for filenames

  echo "===> Collecting from ${NODE} at ${TS_UTC}..."

  # /status
  ssh "${NODE}" "curl -fsS http://127.0.0.1:8080/status" \
    > "${LOG_DIR}/status_${SAFE_NODE}_${TS_UTC}.json" \
    || echo "WARN: Failed to collect /status from ${NODE}"

  # /ai/status
  ssh "${NODE}" "curl -fsS http://127.0.0.1:8080/ai/status" \
    > "${LOG_DIR}/ai_status_${SAFE_NODE}_${TS_UTC}.json" \
    || echo "WARN: Failed to collect /ai/status from ${NODE}"
done

echo "OK: Snapshot written under ${LOG_DIR}/ for timestamp ${TS_UTC}"


