#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
COMPOSE_FILE="${COMPOSE_FILE:-$ROOT_DIR/localnet/docker-compose.full-stack.yaml}"
PROJECT_NAME="${PROJECT_NAME:-ippan-local}"
MODE="${LOCAL_STACK_MODE:-docker}"

usage() {
  cat <<'USAGE'
Usage: scripts/run-local-full-stack.sh [--docker|--native]

Options:
  --docker   Force Docker Compose mode (default)
  --native   Run the three-node localnet without Docker
  -h, --help Show this help message and exit
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --docker)
      MODE="docker"
      shift
      ;;
    --native)
      MODE="native"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option '$1'" >&2
      usage >&2
      exit 1
      ;;
  esac
done

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: '$1' is required but not installed" >&2
    exit 1
  fi
}

run_docker_mode() {
  if [[ ! -f "$COMPOSE_FILE" ]]; then
    echo "error: compose file not found at $COMPOSE_FILE" >&2
    exit 1
  fi

  require_cmd docker

  if docker compose version >/dev/null 2>&1; then
    COMPOSE_BIN=(docker compose)
  elif command -v docker-compose >/dev/null 2>&1; then
    COMPOSE_BIN=(docker-compose)
  else
    echo "error: docker compose plugin or docker-compose binary is required" >&2
    exit 1
  fi

  DATA_DIR="$ROOT_DIR/localnet/data/node"
  mkdir -p "$DATA_DIR"

  echo "▶️  Building and starting the IPPAN local full-stack environment (Docker mode)..."
  "${COMPOSE_BIN[@]}" -f "$COMPOSE_FILE" -p "$PROJECT_NAME" up --build -d

  echo
  "${COMPOSE_BIN[@]}" -f "$COMPOSE_FILE" -p "$PROJECT_NAME" ps

  cat <<'INSTRUCTIONS'

✅ Services are starting in the background.

Visit the following once the containers report "healthy":
  • Node RPC:      http://localhost:8080/health
  • Gateway API:   http://localhost:8081/api/health
  • Unified UI:    http://localhost:3000

Next steps:
  1. Generate a devnet wallet:
       ippan-wallet --network devnet generate-key --out ./keys/dev.key --prompt-password
  2. Fund it via /dev/fund (dev mode) or use the faucet.
  3. Send a payment:
       ippan-wallet --rpc-url http://localhost:8081 send-payment --key ./keys/dev.key --prompt-password --to @friend.ipn --amount 0.1 --memo "hello"
  4. Watch the transaction appear in the explorer UI.

To follow logs:
  ${COMPOSE_BIN[@]} -f localnet/docker-compose.full-stack.yaml -p ippan-local logs -f

To stop everything:
  ${COMPOSE_BIN[@]} -f localnet/docker-compose.full-stack.yaml -p ippan-local down

INSTRUCTIONS
}

run_native_mode() {
  echo "[full-stack] Building IPPAN workspace (release builds recommended for speed)..."
  (cd "$ROOT_DIR" && cargo build --workspace)

  echo "[full-stack] Ensuring any previous localnet is stopped..."
  if [[ -x "$SCRIPT_DIR/localnet_stop.sh" ]]; then
    "$SCRIPT_DIR/localnet_stop.sh" >/dev/null 2>&1 || true
  fi

  echo "[full-stack] Launching three-node localnet (--native mode)"
  "$SCRIPT_DIR/localnet_start.sh"

  cat <<'NATIVE'
[full-stack] Local environment is running.

RPC endpoints:
  - http://127.0.0.1:8080 (node1)
  - http://127.0.0.1:8081 (node2)
  - http://127.0.0.1:8082 (node3)

Use `scripts/localnet_stop.sh` to shut everything down when you are done.
NATIVE
}

case "$MODE" in
  docker)
    run_docker_mode
    ;;
  native)
    run_native_mode
    ;;
  *)
    echo "error: unsupported mode '$MODE'" >&2
    exit 1
    ;;
esac
