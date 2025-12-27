#!/usr/bin/env bash
set -euo pipefail

# Install the single authoritative systemd drop-in for batch ingest on all nodes.
# This script uses the CORRECT env var names from crates/rpc/src/server.rs:
#   - IPPAN_BATCH_MAX_TX_PER_BATCH (NOT IPPAN_BATCH_MAX_TXS!)
#   - IPPAN_BATCH_BODY_LIMIT_BYTES
#   - IPPAN_BATCH_CONCURRENCY_LIMIT
#   - IPPAN_BATCH_QUEUE_CAPACITY
#
# Usage:
#   ./install_batch_ingest_dropin.sh api1.ippan.uk
#   ./install_batch_ingest_dropin.sh api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk

DROPIN_FILE="70-batch-ingest.conf"
DROPIN_DIR="/etc/systemd/system/ippan-node.service.d"
SERVICE="ippan-node"

# Files to disable (rename to .disabled) - prevents drift from multiple sources
DISABLE_PATTERNS=(
  "15-dlc-config.conf"
  "31-dlc-runtime.conf"
  "99-dlc-config-path.conf"
  "99-consensus-validators.conf"
  "99-validator-set.conf"
  "30-bootstrap.conf"
  "99-bootstrap-override.conf"
)

# Batch lane configuration (high-throughput tuned for 200k-500k offered TPS)
BATCH_MAX_TX_PER_BATCH="${BATCH_MAX_TX_PER_BATCH:-4096}"
BATCH_BODY_LIMIT_BYTES="${BATCH_BODY_LIMIT_BYTES:-268435456}"  # 256 MiB
BATCH_CONCURRENCY_LIMIT="${BATCH_CONCURRENCY_LIMIT:-256}"
BATCH_QUEUE_CAPACITY="${BATCH_QUEUE_CAPACITY:-16384}"
BATCH_BACKPRESSURE_MEMPOOL_SIZE="${BATCH_BACKPRESSURE_MEMPOOL_SIZE:-100000}"

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <host1> [host2] [host3] ..."
  echo "Example: $0 api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk"
  exit 1
fi

DROPIN_CONTENT="[Service]
# Single authoritative batch ingest configuration
# Installed by: scripts/ops/install_batch_ingest_dropin.sh
# Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)

# Enable dev batch submit lane
Environment=IPPAN_DEV_MODE=true
Environment=IPPAN_ENABLE_BATCH_SUBMIT=1

# CRITICAL: These key names MUST match crates/rpc/src/server.rs exactly
#   - IPPAN_BATCH_MAX_TX_PER_BATCH (NOT IPPAN_BATCH_MAX_TXS!)
#   - IPPAN_BATCH_BODY_LIMIT_BYTES
#   - IPPAN_BATCH_CONCURRENCY_LIMIT
#   - IPPAN_BATCH_QUEUE_CAPACITY
Environment=IPPAN_BATCH_MAX_TX_PER_BATCH=${BATCH_MAX_TX_PER_BATCH}
Environment=IPPAN_BATCH_BODY_LIMIT_BYTES=${BATCH_BODY_LIMIT_BYTES}
Environment=IPPAN_BATCH_CONCURRENCY_LIMIT=${BATCH_CONCURRENCY_LIMIT}
Environment=IPPAN_BATCH_QUEUE_CAPACITY=${BATCH_QUEUE_CAPACITY}
Environment=IPPAN_BATCH_BACKPRESSURE_MEMPOOL_SIZE=${BATCH_BACKPRESSURE_MEMPOOL_SIZE}

# Optional: make overload immediate and visible
Environment=RUST_LOG=info,ippan_rpc=info
"

install_on_host() {
  local host="$1"
  echo "=== Installing on ${host} ==="

  ssh "root@${host}" "set -e
    # Create drop-in directory
    mkdir -p '${DROPIN_DIR}'

    # Write the authoritative drop-in
    cat > '${DROPIN_DIR}/${DROPIN_FILE}' <<'DROPIN_EOF'
${DROPIN_CONTENT}
DROPIN_EOF

    # Disable drift sources (rename only, preserve history)
    cd '${DROPIN_DIR}'
    for f in ${DISABLE_PATTERNS[*]}; do
      if [ -f \"\$f\" ]; then
        echo \"  Disabling: \$f -> \$f.disabled\"
        mv -f \"\$f\" \"\$f.disabled\"
      fi
    done

    # Show active drop-ins
    echo '  Active drop-ins:'
    ls -1 *.conf 2>/dev/null | sed 's/^/    /' || echo '    (none)'

    # Reload and restart
    systemctl daemon-reload
    systemctl restart '${SERVICE}'
    sleep 2

    # Verify
    if ! systemctl is-active --quiet '${SERVICE}'; then
      echo \"ERROR: ${SERVICE} failed to start on ${host}\" >&2
      systemctl status '${SERVICE}' --no-pager || true
      exit 1
    fi

    echo \"  ${SERVICE} is active\"
    curl -sS -m 2 http://127.0.0.1:8080/health && echo
  "

  echo "=== ${host} OK ==="
  echo
}

for host in "$@"; do
  install_on_host "$host"
done

echo "All hosts configured. Run drift check to verify:"
echo "  ./scripts/ops/check_batch_dropin_drift.sh $*"

