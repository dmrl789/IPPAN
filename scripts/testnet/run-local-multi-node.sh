#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN_PATH="$ROOT_DIR/target/release/ippan-node"
CONFIG_DIR="$ROOT_DIR/testnet"
DATA_ROOT="$ROOT_DIR/testnet/data"
LOG_DIR="$ROOT_DIR/testnet/logs"
EXTRA_ARGS=("$@")

cd "$ROOT_DIR"

echo "[IPPAN] Building ippan-node in release mode (once)..."
cargo build --release -p ippan-node

mkdir -p "$DATA_ROOT/node1" "$DATA_ROOT/node2" "$DATA_ROOT/node3" "$LOG_DIR"

pids=()
cleanup() {
  if [ ${#pids[@]} -gt 0 ]; then
    echo "\n[IPPAN] Stopping RC testnet nodes..."
    kill "${pids[@]}" 2>/dev/null || true
    wait "${pids[@]}" 2>/dev/null || true
  fi
}
trap cleanup INT TERM EXIT

start_node() {
  local name="$1"
  local config="$2"
  local data_dir="$3"

  echo "[IPPAN] Launching $name using $config"
  "$BIN_PATH" --config "$config" --data-dir "$data_dir" "${EXTRA_ARGS[@]}" >"$LOG_DIR/${name}.log" 2>&1 &
  pids+=("$!")
}

start_node node1 "$CONFIG_DIR/node1.toml" "$DATA_ROOT/node1"
start_node node2 "$CONFIG_DIR/node2.toml" "$DATA_ROOT/node2"
start_node node3 "$CONFIG_DIR/node3.toml" "$DATA_ROOT/node3"

echo "[IPPAN] Three-node RC testnet is running. Logs: $LOG_DIR/node{1,2,3}.log"
echo "[IPPAN] Press Ctrl+C to stop all nodes."

wait
