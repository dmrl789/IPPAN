#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN_PATH="$ROOT_DIR/target/release/ippan-node"
CONFIG_PATH="$ROOT_DIR/testnet/node1.toml"
DATA_DIR="$ROOT_DIR/testnet/data/node1"
LOG_DIR="$ROOT_DIR/testnet/logs"
LOG_FILE="$LOG_DIR/node1.log"

cd "$ROOT_DIR"

echo "[IPPAN] Building ippan-node in release mode (once)..."
cargo build --release -p ippan-node

mkdir -p "$DATA_DIR" "$LOG_DIR"

echo "[IPPAN] Starting single-node RC testnet"
echo "         config: $CONFIG_PATH"
echo "         data:   $DATA_DIR"
echo "         log:    $LOG_FILE"

echo "[IPPAN] Press Ctrl+C to stop the node. Logs are being tee'd to $LOG_FILE"
"$BIN_PATH" --config "$CONFIG_PATH" --data-dir "$DATA_DIR" "$@" 2>&1 | tee "$LOG_FILE"
