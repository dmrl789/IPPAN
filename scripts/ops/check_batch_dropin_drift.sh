#!/usr/bin/env bash
set -euo pipefail

# Drift-killer check: fail if multiple active drop-ins define batch/validator knobs.
#
# Usage:
#   ./check_batch_dropin_drift.sh api1.ippan.uk
#   ./check_batch_dropin_drift.sh api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk

DROPIN_DIR="/etc/systemd/system/ippan-node.service.d"

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <host1> [host2] [host3] ..."
  exit 1
fi

FAILED=0

check_host() {
  local host="$1"
  echo "=== Checking ${host} ==="

  local result
  result=$(ssh "root@${host}" "set -e
    cd '${DROPIN_DIR}' 2>/dev/null || { echo 'NO_DROPIN_DIR'; exit 0; }

    # Find all active .conf files (not .disabled)
    active_confs=\$(ls -1 *.conf 2>/dev/null || true)
    if [ -z \"\$active_confs\" ]; then
      echo 'NO_ACTIVE_DROPINS'
      exit 0
    fi

    # Check for IPPAN_BATCH_ definitions across all active files
    batch_defs=\$(grep -l 'IPPAN_BATCH_' *.conf 2>/dev/null || true)
    batch_count=\$(echo \"\$batch_defs\" | grep -c '.' || true)

    # Check for IPPAN_VALIDATOR definitions across all active files
    validator_defs=\$(grep -l 'IPPAN_VALIDATOR' *.conf 2>/dev/null || true)
    validator_count=\$(echo \"\$validator_defs\" | grep -c '.' || true)

    echo \"ACTIVE_DROPINS:\$active_confs\"
    echo \"BATCH_SOURCES:\$batch_count:\$batch_defs\"
    echo \"VALIDATOR_SOURCES:\$validator_count:\$validator_defs\"

    # Show the actual content of 70-batch-ingest.conf for verification
    if [ -f '70-batch-ingest.conf' ]; then
      echo 'BATCH_CONFIG_CONTENT:'
      grep 'Environment=IPPAN_BATCH_' '70-batch-ingest.conf' || true
    fi
  ")

  echo "$result"

  # Parse batch sources count
  local batch_sources
  batch_sources=$(echo "$result" | grep '^BATCH_SOURCES:' | cut -d: -f2 || echo "0")

  if [[ "$batch_sources" -gt 1 ]]; then
    echo "ERROR: Multiple files define IPPAN_BATCH_ on ${host}!" >&2
    FAILED=1
  fi

  # Verify the correct env var name is used
  if echo "$result" | grep -q 'IPPAN_BATCH_MAX_TXS='; then
    echo "ERROR: Wrong env var IPPAN_BATCH_MAX_TXS detected on ${host}!" >&2
    echo "       Should be IPPAN_BATCH_MAX_TX_PER_BATCH" >&2
    FAILED=1
  fi

  echo
}

for host in "$@"; do
  check_host "$host"
done

if [[ "$FAILED" -ne 0 ]]; then
  echo "DRIFT DETECTED! Fix by disabling extra drop-ins and ensuring correct env var names."
  exit 1
fi

echo "All hosts clean. No drift detected."

