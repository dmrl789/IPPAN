#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
CONFIG_DIR="${ROOT_DIR}/localnet"

stop_node() {
  local name="$1"
  local pid_file="${CONFIG_DIR}/${name}.pid"
  if [[ ! -f "${pid_file}" ]]; then
    echo "[localnet] No PID file for ${name}" >&2
    return
  fi
  local pid
  pid="$(cat "${pid_file}")"
  if [[ -z "${pid}" ]]; then
    rm -f "${pid_file}"
    return
  fi
  if [[ ! -e "/proc/${pid}" ]]; then
    echo "[localnet] ${name} (PID ${pid}) already stopped" >&2
    rm -f "${pid_file}"
    return
  fi
  echo "[localnet] Stopping ${name} (PID ${pid})" >&2
  kill "${pid}" >/dev/null 2>&1 || true
  sleep 1
  if kill -0 "${pid}" >/dev/null 2>&1; then
    echo "[localnet] ${name} still running, sending SIGKILL" >&2
    kill -9 "${pid}" >/dev/null 2>&1 || true
  fi
  rm -f "${pid_file}"
  echo "[localnet] ${name} stopped" >&2
}

stop_node node1
stop_node node2
stop_node node3

echo "[localnet] Requested shutdown for all nodes." >&2
