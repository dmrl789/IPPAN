#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LOG_DIR="$ROOT_DIR/scripts/perf/logs"
CONFIG_PATH=${CONFIG_PATH:-"$ROOT_DIR/config/local-node.toml"}

mkdir -p "$LOG_DIR"

export IPPAN_DEV_MODE=${IPPAN_DEV_MODE:-true}
export RUST_LOG=${RUST_LOG:-info}

echo "[perf] building node in release mode..."
cargo build -p node --release

echo "[perf] starting node using $CONFIG_PATH"
"$ROOT_DIR/target/release/node" --config "$CONFIG_PATH" --dev \
  >"$LOG_DIR/node.log" 2>&1
