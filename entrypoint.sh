#!/bin/sh
set -euxo pipefail

: "${IPPAN_CONFIG_DIR:=/app/data}"
mkdir -p "$IPPAN_CONFIG_DIR" || true
chown -R "$(id -u)":"$(id -g)" "$IPPAN_CONFIG_DIR" 2>/dev/null || true

export RUST_LOG="${RUST_LOG:-info}"
export RUST_BACKTRACE="${RUST_BACKTRACE:-full}"

echo "[entrypoint] IPPAN_CONFIG_DIR=$IPPAN_CONFIG_DIR"
echo "[entrypoint] RUST_LOG=$RUST_LOG RUST_BACKTRACE=$RUST_BACKTRACE"

exec /usr/local/bin/ippan
